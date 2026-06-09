use std::sync::Arc;

use anyhow::Result;
use qrcode::QrCode;
use qrcode::render::unicode;
use wacore::proto_helpers::MessageExt;
use wacore::store::DevicePropsOverride;
use wacore::types::events::Event;
use waproto::whatsapp as wa;
use waproto::whatsapp::device_props::PlatformType;
use whatsapp_rust::bot::Bot;
use whatsapp_rust::store::SqliteStore;
use whatsapp_rust::TokioRuntime;
use whatsapp_rust_tokio_transport::TokioWebSocketTransportFactory;
use whatsapp_rust_ureq_http_client::UreqHttpClient;

use crate::config::AppConfig;

pub async fn start() -> Result<()> {
    // Initialize storage backend
    let whatsapp_storage = AppConfig::internal().gateways_dir.join("whatsapp.db").into_string().unwrap();
    let backend = Arc::new(SqliteStore::new(&whatsapp_storage).await?);

    // Set device name
    let device = DevicePropsOverride::new()
        .with_platform_type(PlatformType::Desktop)
        .with_os("Mia Agent");

    // Build the bot
    let mut bot = Bot::builder()
        .with_device_props(device)
        .with_backend(backend)
        .with_transport_factory(TokioWebSocketTransportFactory::new())
        .with_http_client(UreqHttpClient::new())
        .with_runtime(TokioRuntime)
        .on_event(|event, client| async move {
            match &*event {
                // Pairing
                Event::PairingQrCode { code, .. } => {
                    let qrcode = QrCode::new(code.as_bytes()).unwrap();
                    let image = qrcode.render::<unicode::Dense1x2>()
                        .build();
                    println!("Scan this QR code with WhatsApp:\n{}", image);
                }
                Event::PairingCode { code, timeout } => {
                    println!("WhatsApp pairing code: [{:?}], valid for: {:?}sec", code, timeout.as_secs());
                }
                Event::PairError(e) => {
                    println!("WhatsApp pairing error: {:?}", e);
                }
                Event::PairSuccess(s) => {
                    println!("WhatsApp pairing success: {:?}", s);
                }

                // Connect / Disconnect
                Event::Connected(c) => {
                    println!("WhatsApp connected: {:?}", c);
                }
                Event::Disconnected(d) => {
                    println!("WhatsApp disconnected: {:?}", d);
                }
                Event::ConnectFailure(f) => {
                    println!("WhatsApp connect failure: {:?}", f);
                }
                Event::LoggedOut(l) => {
                    println!("WhatsApp logged out: {:?}", l);
                }
                
                // Message
                Event::Message(msg, info) => {
                    println!("WhatsApp message received");
                    println!("From: {} ({})", info.source.sender.clone(), info.source.chat.clone());
                    if let Some(text) = msg.text_content() {
                        println!("Text: {}", text);
                        if text == "ping" {
                            println!("sending pong");
                            let message = wa::Message {
                                conversation: Some("pong".to_string()),
                                ..Default::default()
                            };
                            match client.send_message(info.source.chat.clone(), message).await {
                                Ok(_) => println!("Successfully sent pong!"),
                                Err(e) => eprintln!("Failed to send whatsapp message back: {:?}", e),
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .build()
        .await?;

    // Start the bot
    bot.run().await?.await?;
    Ok(())
}
