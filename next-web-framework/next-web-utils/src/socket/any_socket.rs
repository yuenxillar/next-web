use std::{io::Read, net::SocketAddr};

pub enum AnySocket {
    Udp(std::net::UdpSocket),
    Tcp(std::net::TcpListener),
}

#[derive(Clone)]
pub enum SocketTy {
    Udp,
    Tcp,
}

impl AnySocket {
    pub fn new(addr: &str, ty: SocketTy) -> Result<AnySocket, std::io::Error> {
        let socket = match ty {
            SocketTy::Udp => {
                let socket = std::net::UdpSocket::bind(addr)?;
                AnySocket::Udp(socket)
            }
            SocketTy::Tcp => {
                let listener = std::net::TcpListener::bind(addr)?;
                AnySocket::Tcp(listener)
            }
        };
        Ok(socket)
    }

    pub fn listen<F>(&self, buff_size: usize, handle: F)
    where
        F: Fn(ReadResult) + Send + 'static,
        F: Clone,
    {
        match self {
            AnySocket::Udp(udp_socket) => {
                let mut buf = Vec::with_capacity(buff_size);
                while let Ok((len, socket_addr)) = udp_socket.recv_from(&mut buf) {
                    handle(ReadResult {
                        len,
                        socket_addr,
                        buf: &buf[..len],
                    });
                }
            }
            AnySocket::Tcp(tcp_listener) => {
                while let Ok((mut stream, socket_addr)) = tcp_listener.accept() {
                    let handle = handle.clone();
                    let mut buf = vec![0u8; buff_size];
                    std::thread::spawn(move || {
                        while let Ok(len) = stream.read(&mut buf) {
                            handle(ReadResult {
                                len,
                                socket_addr,
                                buf: &buf[..len],
                            });
                        }
                    });
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct ReadResult<'a> {
    pub len: usize,
    pub socket_addr: SocketAddr,
    pub buf: &'a [u8],
}

mod test {

    #[test]
    fn test_udp_socket_listener() {
        let udp = super::AnySocket::new("127.0.0.1:6666", super::SocketTy::Tcp).unwrap();
        udp.listen(512, |r| {
            println!(
                "len: {}, socket_addr: {}, buf: {:?}",
                r.len, r.socket_addr, r.buf
            );
        });
    }

    #[test]
    fn test_1() {
        let addr = std::net::ToSocketAddrs::to_socket_addrs(&"www.baidu.com:80")
            .unwrap()
            .next()
            .unwrap();
        println!("addr: {:?}", addr);
    }
}
