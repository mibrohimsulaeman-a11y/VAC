// Narrow conversion helpers for approval-related app-server payloads.
//
// The TUI mostly keeps app-server approval types intact. These helpers cover
// the remaining cases where the UI consumes a private file-change display
// model or needs to translate a granted permission response for outbound
// submission.

use crate::diff_model::FileChange;
use crate::session_protocol::AdditionalNetworkPermissions;
use crate::session_protocol::AppServerNetworkApprovalContext;
use crate::session_protocol::AppServerNetworkApprovalProtocol;
use crate::session_protocol::FileUpdateChange;
use crate::session_protocol::GrantedPermissionProfile;
use crate::session_protocol::NetworkApprovalContext as CoreNetworkApprovalContext;
use crate::session_protocol::NetworkApprovalProtocol as CoreNetworkApprovalProtocol;
use crate::session_protocol::PatchChangeKind;
use std::collections::HashMap;
use std::path::PathBuf;
use vac_protocol::request_permissions::RequestPermissionProfile as CoreRequestPermissionProfile;

pub(crate) fn network_approval_context_from_app_server(
    value: AppServerNetworkApprovalContext,
) -> CoreNetworkApprovalContext {
    CoreNetworkApprovalContext {
        host: value.host,
        protocol: match value.protocol {
            AppServerNetworkApprovalProtocol::Http => CoreNetworkApprovalProtocol::Http,
            AppServerNetworkApprovalProtocol::Https => CoreNetworkApprovalProtocol::Https,
            AppServerNetworkApprovalProtocol::Socks5Tcp => CoreNetworkApprovalProtocol::Socks5Tcp,
            AppServerNetworkApprovalProtocol::Socks5Udp => CoreNetworkApprovalProtocol::Socks5Udp,
        },
    }
}

pub(crate) fn granted_permission_profile_from_request(
    value: CoreRequestPermissionProfile,
) -> GrantedPermissionProfile {
    GrantedPermissionProfile {
        network: value.network.map(|network| AdditionalNetworkPermissions {
            enabled: network.enabled,
        }),
        file_system: value.file_system.map(Into::into),
    }
}

pub(crate) fn file_update_changes_to_display(
    changes: Vec<FileUpdateChange>,
) -> HashMap<PathBuf, FileChange> {
    changes
        .into_iter()
        .map(|change| {
            let path = PathBuf::from(change.path);
            let file_change = match change.kind {
                PatchChangeKind::Add => FileChange::Add {
                    content: change.diff,
                },
                PatchChangeKind::Delete => FileChange::Delete {
                    content: change.diff,
                },
                PatchChangeKind::Update { move_path } => FileChange::Update {
                    unified_diff: change.diff,
                    move_path,
                },
            };
            (path, file_change)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::file_update_changes_to_display;
    use super::granted_permission_profile_from_request;
    use super::network_approval_context_from_app_server;
    use crate::diff_model::FileChange;
    use crate::session_protocol::AppServerNetworkApprovalContext;
    use crate::session_protocol::AppServerNetworkApprovalProtocol;
    use crate::session_protocol::FileUpdateChange;
    use crate::session_protocol::NetworkApprovalContext as CoreNetworkApprovalContext;
    use crate::session_protocol::NetworkApprovalProtocol as CoreNetworkApprovalProtocol;
    use crate::session_protocol::PatchChangeKind;
    use pretty_assertions::assert_eq;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use vac_utils_absolute_path::AbsolutePathBuf;

    fn absolute_path(path: &str) -> AbsolutePathBuf {
        AbsolutePathBuf::try_from(PathBuf::from(path)).expect("path must be absolute")
    }

    #[test]
    fn converts_network_approval_context_from_app_server() {
        assert_eq!(
            network_approval_context_from_app_server(AppServerNetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: AppServerNetworkApprovalProtocol::Socks5Tcp,
            }),
            CoreNetworkApprovalContext {
                host: "example.com".to_string(),
                protocol: CoreNetworkApprovalProtocol::Socks5Tcp,
            }
        );
    }

    #[test]
    fn converts_file_update_changes_to_display() {
        assert_eq!(
            file_update_changes_to_display(vec![FileUpdateChange {
                path: "foo.txt".to_string(),
                kind: PatchChangeKind::Add,
                diff: "hello\n".to_string(),
            }]),
            HashMap::from([(
                PathBuf::from("foo.txt"),
                FileChange::Add {
                    content: "hello\n".to_string(),
                },
            )])
        );
    }

    #[test]
    fn converts_request_permissions_into_granted_permissions() {
        assert_eq!(
            granted_permission_profile_from_request(crate::session_protocol::RequestPermissionProfile {
                    network: Some(
                        crate::session_protocol::AdditionalNetworkPermissions {
                            enabled: Some(true),
                        }
                        .into()
                    ),
                    file_system: Some(
                        crate::session_protocol::AdditionalFileSystemPermissions {
                            read: Some(vec![absolute_path("/tmp/read-only")]),
                            write: Some(vec![absolute_path("/tmp/write")]),
                            glob_scan_max_depth: None,
                            entries: None,
                        }
                        .into()
                    ),
                }),
            crate::session_protocol::GrantedPermissionProfile {
                network: Some(crate::session_protocol::AdditionalNetworkPermissions {
                    enabled: Some(true),
                }),
                file_system: Some(crate::session_protocol::AdditionalFileSystemPermissions {
                    read: Some(vec![absolute_path("/tmp/read-only")]),
                    write: Some(vec![absolute_path("/tmp/write")]),
                    glob_scan_max_depth: None,
                    entries: Some(vec![
                        crate::session_protocol::FileSystemSandboxEntry {
                            path: crate::session_protocol::FileSystemPath::Path {
                                path: absolute_path("/tmp/read-only"),
                            },
                            access: crate::session_protocol::FileSystemAccessMode::Read,
                        },
                        crate::session_protocol::FileSystemSandboxEntry {
                            path: crate::session_protocol::FileSystemPath::Path {
                                path: absolute_path("/tmp/write"),
                            },
                            access: crate::session_protocol::FileSystemAccessMode::Write,
                        },
                    ]),
                }),
            }
        );
    }

    #[test]
    fn converts_request_permissions_into_canonical_granted_permissions() {
        assert_eq!(
            granted_permission_profile_from_request(crate::session_protocol::RequestPermissionProfile {
                    network: None,
                    file_system: Some(
                        crate::session_protocol::AdditionalFileSystemPermissions {
                            read: None,
                            write: None,
                            glob_scan_max_depth: None,
                            entries: Some(vec![crate::session_protocol::FileSystemSandboxEntry {
                                path: crate::session_protocol::FileSystemPath::Special {
                                    value: crate::session_protocol::FileSystemSpecialPath::Root,
                                },
                                access: crate::session_protocol::FileSystemAccessMode::Write,
                            }]),
                        }
                        .into()
                    ),
                }),
            crate::session_protocol::GrantedPermissionProfile {
                network: None,
                file_system: Some(crate::session_protocol::AdditionalFileSystemPermissions {
                    read: None,
                    write: None,
                    glob_scan_max_depth: None,
                    entries: Some(vec![crate::session_protocol::FileSystemSandboxEntry {
                        path: crate::session_protocol::FileSystemPath::Special {
                            value: crate::session_protocol::FileSystemSpecialPath::Root,
                        },
                        access: crate::session_protocol::FileSystemAccessMode::Write,
                    },]),
                }),
            }
        );
    }
}
