use std::collections::HashSet;

use ix_core::error::{IxError, Result};
use ix_core::item::{Category, Item};
use ix_core::{Context, Provider};
use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo};

#[derive(Default)]
pub struct PortProvider;

impl PortProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Provider for PortProvider {
    fn name(&self) -> &str {
        "port"
    }

    fn detect(&self, _ctx: &Context) -> bool {
        true
    }

    fn list(&self, ctx: &Context) -> Result<Vec<Item>> {
        let show_all = ctx.has_flag("a") || ctx.has_flag("all");

        let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
        let proto_flags = if show_all {
            ProtocolFlags::TCP | ProtocolFlags::UDP
        } else {
            ProtocolFlags::TCP
        };

        let sockets = get_sockets_info(af_flags, proto_flags)
            .map_err(|e| IxError::Provider(format!("netstat2: {e}")))?;

        let mut items = Vec::new();
        let mut seen = HashSet::new();

        for socket in sockets {
            let (port, proto, state) = match socket.protocol_socket_info {
                ProtocolSocketInfo::Tcp(tcp) => {
                    // Only list listening TCP ports
                    if tcp.state != netstat2::TcpState::Listen {
                        continue;
                    }
                    (tcp.local_port, "tcp", "LISTEN")
                }
                ProtocolSocketInfo::Udp(udp) => (udp.local_port, "udp", "OPEN"),
            };

            // Deduplicate by port + proto
            if !seen.insert((port, proto)) {
                continue;
            }

            let raw = port.to_string();

            // Format process info if available
            let mut proc_info = String::new();
            let mut pids = socket.associated_pids;
            pids.sort_unstable();
            pids.dedup();

            if !pids.is_empty() {
                // If we have sysinfo, we could lookup names, but just PIDs for now
                // is often good enough and avoids a heavy sysinfo dependency inside ix-port.
                let pids_str = pids
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                proc_info = format!("  (pid {pids_str})");
            }

            let label = format!(":{port}{proc_info}");

            let item = Item::new(0, raw, label)
                .with_status(state, Category::Positive)
                .with_group(proto);

            items.push(item);
        }

        items.sort_unstable_by(|a, b| {
            // Because raw is the port number string we have to parse it to sort numerically
            let a_port = a.raw.parse::<u16>().unwrap_or(0);
            let b_port = b.raw.parse::<u16>().unwrap_or(0);
            a_port.cmp(&b_port).then_with(|| a.group.cmp(&b.group))
        });

        Ok(items)
    }

    fn preview_cmd(&self, item: &Item) -> Option<String> {
        Some(format!("lsof -i :{}", item.raw))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_core::test_utils::ctx_default;
    use std::net::{TcpListener, UdpSocket};

    #[test]
    fn test_port_finds_tcp_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        let items = PortProvider::new().list(&ctx_default()).unwrap();

        assert!(
            items
                .iter()
                .any(|i| i.raw == port.to_string() && i.group.as_deref() == Some("tcp")),
            "expected to find tcp port {port}"
        );
    }

    #[test]
    fn test_port_hides_udp_by_default() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();

        let items = PortProvider::new().list(&ctx_default()).unwrap();

        assert!(
            !items
                .iter()
                .any(|i| i.raw == port.to_string() && i.group.as_deref() == Some("udp")),
            "expected to NOT find udp port {port} without flags"
        );
    }

    #[test]
    fn test_port_shows_udp_with_all_flag() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = socket.local_addr().unwrap().port();

        let mut ctx = ctx_default();
        ctx.flags.insert("all".into());
        let items = PortProvider::new().list(&ctx).unwrap();

        assert!(
            items
                .iter()
                .any(|i| i.raw == port.to_string() && i.group.as_deref() == Some("udp")),
            "expected to find udp port {port}"
        );
    }
}
