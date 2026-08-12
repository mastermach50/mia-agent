use bytesize::ByteSize;
use chrono::{DateTime, Local};
use indoc::indoc;
use log::warn;
use serde_json::json;
use std::{fs, io};
use tabled::settings::Style;

use crate::{agent_loop::AgentHandle, agent_tools::Tool};

#[derive(Debug)]
pub struct FSListDir;
#[async_trait::async_trait]
impl Tool for FSListDir {
    fn name(&self) -> String {
        "fs_list_dir".to_string()
    }
    fn icon(&self) -> String {
        "📂".to_string()
    }
    fn short(&self, args: serde_json::Value) -> String {
        args["path"].as_str().unwrap_or(".").to_string()
    }
    fn availability(&self) -> Result<(), String> {
        Ok(())
    }
    fn schema(&self) -> serde_json::Value {
        let description = indoc! {"
        List the contents of a directory with full metadata: permissions, owner, group, file size, and last modification time.
        Use before reading or modifying unknown directories, to audit what files are present, or when the user asks about directory structure.
        Defaults to the current directory if no path is given.
        "};
        json!({
            "type": "function",
            "function": {
                "name": &self.name(),
                "description": description,
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "The folder path to list (relative to current directory, defaults to . )"
                        },
                    },
                    "required": []
                }
            }
        })
    }
    async fn execute(&self, handle: &AgentHandle, args: serde_json::Value) -> serde_json::Value {
        let path = shellexpand::tilde(args["path"].as_str().unwrap_or(".")).to_string();

        match fs::read_dir(&path) {
            Ok(entries) => {
                let mut table = tabled::builder::Builder::new();
                table.push_record(["Permission", "Size", "User", "Group", "Modified", "Path"]);
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            let path_disp = path.display().to_string();

                            if let Ok(metadata) = entry.metadata() {
                                let perms = permission_string(&metadata);
                                let size = ByteSize::b(metadata.len()).to_string();
                                let (owner, group) = owner_group_string(&metadata);
                                let modified = metadata
                                    .modified()
                                    .map(|t| {
                                        DateTime::<Local>::from(t)
                                            .format("%Y-%m-%d %H:%M:%S")
                                            .to_string()
                                    })
                                    .unwrap_or("-".to_string());
                                table.push_record([perms, size, owner, group, modified, path_disp]);
                            } else {
                                warn!("Failed to read metadata for {path_disp}\n");
                                table.push_record(["[Failed to get metadata]", "", "", "", "", &path_disp]);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to read directory entry: {:?}", e);
                            table.push_record(["[Failed to read entry]"]);
                        }
                    }
                }

                let record_count = table.count_records();
                let string_table = table.build().with(Style::empty()).to_string();

                handle.tool_output(&string_table, String::new());

                return json!({
                    "status": "success",
                    "output": &string_table,
                    "count": record_count
                });
            }
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => {
                    return json!({
                        "status": "error",
                        "message": format!("Directory not found: {}", path)
                    });
                }
                io::ErrorKind::PermissionDenied => {
                    return json!({
                        "status": "error",
                        "message": format!("Permission denied: {}", path)
                    });
                }
                io::ErrorKind::NotADirectory => {
                    return json!({
                        "status": "error",
                        "message": format!("Not a directory: {}", path)
                    });
                }
                _ => {
                    warn!("Unknown error: {:?}", e);
                    return json!({
                        "status": "error",
                        "message": format!("Unknown error: {}", e)
                    });
                }
            },
        }
    }
}

#[cfg(unix)]
fn permission_string(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;
    use unix_mode;

    unix_mode::to_string(metadata.mode())
}

#[cfg(windows)]
fn permission_string(metadata: &fs::Metadata) -> String {
    let dir_str = if metadata.is_dir() {
        "d".to_string()
    } else {
        "-".to_string()
    };
    let perm_str = if metadata.permissions().readonly() {
        "r-".to_string()
    } else {
        "rw".to_string()
    };
    dir_str + &perm_str + "-------"
}

#[cfg(unix)]
fn owner_group_string(metadata: &fs::Metadata) -> (String, String) {
    use std::os::unix::fs::MetadataExt;
    use uzers;

    let user = uzers::get_user_by_uid(metadata.uid())
        .map(|u| u.name().to_string_lossy().to_string())
        .unwrap_or("-".to_string());
    let group = uzers::get_group_by_gid(metadata.gid())
        .map(|g| g.name().to_string_lossy().to_string())
        .unwrap_or("-".to_string());
    (user, group)
}

#[cfg(windows)]
fn owner_group_string(_metadata: &fs::Metadata) -> (String, String) {
    ("-".to_string(), "-".to_string())
}
