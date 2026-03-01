// osc_handler.rs - VRChat OSC通信による公開範囲変更

use rosc::{decoder, encoder, OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::mpsc;
use std::thread;

const OSC_PARAM: &str = "/avatar/parameters/EterPixVisibility";

/// OSC int値 → visibility文字列
pub fn osc_to_visibility(value: i32) -> Option<&'static str> {
    match value {
        2 => Some("self"),
        3 => Some("friends"),
        4 => Some("instance_friends"),
        5 => Some("instance"),
        6 => Some("public"),
        _ => None,
    }
}

/// visibility文字列 → OSC int値
pub fn visibility_to_osc(visibility: &str) -> i32 {
    match visibility {
        "self" => 2,
        "friends" => 3,
        "instance_friends" => 4,
        "instance" => 5,
        "public" => 6,
        _ => 2,
    }
}

/// OSCイベント (受信→メインスレッド通知)
pub enum OscEvent {
    VisibilityChanged(String),
}

pub struct OscHandler {
    send_port: u16,
    recv_port: u16,
    host: String,
    pub is_running: bool,
    pub current_visibility: String,
    pub last_recv_value: i32,
    stop_tx: Option<mpsc::Sender<()>>,
    send_socket: Option<UdpSocket>,
}

impl OscHandler {
    pub fn new(send_port: u16, recv_port: u16) -> Self {
        Self {
            send_port,
            recv_port,
            host: "127.0.0.1".to_string(),
            is_running: false,
            current_visibility: "self".to_string(),
            last_recv_value: 0,
            stop_tx: None,
            send_socket: None,
        }
    }

    /// OSC通信開始
    pub fn start(
        &mut self,
        event_tx: mpsc::Sender<OscEvent>,
    ) -> anyhow::Result<()> {
        if self.is_running {
            return Ok(());
        }

        // 送信ソケット
        let send_sock = UdpSocket::bind("0.0.0.0:0")?;
        send_sock.connect(format!("{}:{}", self.host, self.send_port))?;
        self.send_socket = Some(send_sock);

        // 受信スレッド
        let recv_addr = format!("{}:{}", self.host, self.recv_port);
        let recv_sock = UdpSocket::bind(&recv_addr)?;
        recv_sock.set_read_timeout(Some(std::time::Duration::from_millis(500)))?;

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        self.stop_tx = Some(stop_tx);

        let current_vis = self.current_visibility.clone();
        // 受信スレッド起動
        let send_sock_clone = UdpSocket::bind("0.0.0.0:0")?;
        send_sock_clone.connect(format!("{}:{}", self.host, self.send_port))?;

        thread::spawn(move || {
            let mut current = current_vis;
            let mut buf = [0u8; 1024];
            loop {
                // 停止チェック
                if stop_rx.try_recv().is_ok() {
                    break;
                }
                match recv_sock.recv(&mut buf) {
                    Ok(size) => {
                        if let Ok((_, OscPacket::Message(msg))) = decoder::decode_udp(&buf[..size]) {
                            if msg.addr == OSC_PARAM {
                                if let Some(OscType::Int(value)) = msg.args.into_iter().next() {
                                    // 0: 無効/変数なし → 無視
                                    if value == 0 {
                                        continue;
                                    }
                                    // 1: アバターリセット → 現在値を再送
                                    if value == 1 {
                                        let _ = send_osc_int(&send_sock_clone, &current);
                                        continue;
                                    }
                                    // 2-6: 有効値
                                    if let Some(new_vis) = osc_to_visibility(value) {
                                        if new_vis != current {
                                            current = new_vis.to_string();
                                            let _ = event_tx.send(OscEvent::VisibilityChanged(new_vis.to_string()));
                                            // 確認のため送り返す
                                            let _ = send_osc_int(&send_sock_clone, &current);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(_) => break,
                }
            }
        });

        self.is_running = true;
        Ok(())
    }

    /// OSC通信停止
    pub fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        self.send_socket = None;
        self.is_running = false;
    }

    /// 公開範囲をVRCに送信
    pub fn send_visibility(&mut self, visibility: &str) {
        self.current_visibility = visibility.to_string();
        if let Some(sock) = &self.send_socket {
            let _ = send_osc_int(sock, visibility);
        }
    }
}

fn send_osc_int(sock: &UdpSocket, visibility: &str) -> anyhow::Result<()> {
    let value = visibility_to_osc(visibility);
    let msg = OscPacket::Message(OscMessage {
        addr: OSC_PARAM.to_string(),
        args: vec![OscType::Int(value)],
    });
    let encoded = encoder::encode(&msg)?;
    sock.send(&encoded)?;
    Ok(())
}
