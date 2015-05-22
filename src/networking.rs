extern crate time;

use std::net::TcpStream;
use std::net::UdpSocket;
use std::io::prelude::*;
use std::vec::Vec;
use std::thread;

use std::net::SocketAddrV4;
use std::net::Ipv4Addr;

pub fn discover_hosts() -> Result<Vec<(String, String)>, &'static str>
{
    let socket_ = UdpSocket::bind("0.0.0.0:3629");
    if socket_.is_err()
    {
        return Err("socket_bind_failed");
    }
    let socket = socket_.unwrap();
    for i in 1..255 // Broadcast doesn't work with the Monash network, so I must do this.
    {
        let _ = socket.send_to("ESC/VP.net\x10\x01\x00\x00\x00\x00".as_bytes(), SocketAddrV4::new(Ipv4Addr::new(118, 139, 125, i), 3629));
        thread::sleep_ms(50); // If I do it too fast, nothing comes back. Stupid firewall.
    }
    let buf: &mut [u8] = &mut [0; 1024];
    
    let mut ret = Vec::<(String, String)>::new();
    thread::sleep_ms(5000); // Wait 5s for all responses to come in.
    let _ = socket.send_to("\x45\x45".as_bytes(), "127.0.0.1:3629"); // This packet indicates that it is time to stop listening.
    
    loop
    {
        //println!("Running main loop...");
        let sock_result = socket.recv_from(buf);
        if sock_result.is_err()
        {
            return Err("could_not_receive");
        }
        let (amt, src) = sock_result.unwrap();
        
        if (buf[0] == 0x45) && (buf[1] == 0x45)
        {
            break; // Don't read any more packets.
        }
        else if amt < 34
        {
            println!("[debug] response too short");
        }
        else if buf[0..10] != "ESC/VP.net".as_bytes()[..]
        {
            println!("[debug] wrong protocol");
        }
        else if buf[14] != 0x20
        {
            println!("[debug] Non-OK response code: {}", buf[14]);
        }
        else
        {
            let mut name = Vec::new();
            for i in 18..35
            {
                if (buf[i] > 31) && (buf[i] < 127)
                {
                    name.push(buf[i]);
                }
                else
                {
                    break; // Stop reading after hitting a non-printable character. Hack-y, but it works.
                }
            }
            ret.push((format!("{}", src), String::from_utf8(name).unwrap()));
        }
    }
    Ok(ret)
}

pub fn connect_tcp(addr: &str) -> Result<TcpStream, &'static str>
{
    let buf: &mut [u8] = &mut [0; 1024];
    let stream_ = TcpStream::connect(addr);
    
    if stream_.is_ok()
    {
        let mut stream = stream_.unwrap();
        
        let _ = stream.write("ESC/VP.net\x10\x03\x00\x00\x00\x00".as_bytes());
        let _ = stream.read(buf);
    
        if buf[0..10] == "ESC/VP.net".as_bytes()[..] // Actually, this goes up to index 9.
        {
            match buf[14]
            {
                0x20 => Ok(stream), // This is the only option that keeps the stream from going out of scope.
                0x41 => Err("password_required"),
                0x43 => Err("wrong_password"),
                0x53 => Err("busy"),
                _ => Err("response_code_not_recognised"),
            }
        } else {
            // This is not ESC/VP.net.
            Err("wrong_protocol")
        }
    } else {
        Err("connection_failed")
    }
}

pub fn send_command<'a>(addr: &'a str, command: &'a str) -> Result<String, &'a str>
{
    let buf: &mut [u8] = &mut [0; 1024];
    let stream_ = connect_tcp(addr);
    
    if stream_.is_ok()
    {
        let mut stream = stream_.unwrap();
        let _ = stream.write(command.as_bytes());
        let _ = stream.write("\r".as_bytes());
        let _ = stream.read(&mut (*buf));
        
        Ok(String::from_utf8_lossy(buf).replace("\r:", ""))
        
    } else {
        match stream_
        {
            Err(e) => Err(e),
            Ok(_) => unreachable!(),
        }
    }
}
