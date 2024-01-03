use clap::{Arg, Command};
use flate2::write::ZlibEncoder;
use reqwest::Client;
use std::fs::File;
use std::io::{self, Read, Write};
use std::convert::TryInto;
use tokio::io::AsyncReadExt;

async fn compress_file(file_path: &str) -> Result<Vec<u8>, io::Error> {
    let mut file = File::open(file_path)?;
    let mut file_content = Vec::new();
    file.read_to_end(&mut file_content)?;

    let mut encoder = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&file_content)?;
    encoder.finish()
}

async fn send_compressed_data(
    compressed_data: Vec<u8>,
    receiver_url: &str,
    sender_email: &str,
) -> Result<(), reqwest::Error> {
    let client = Client::new();

    let response = client
        .post(receiver_url)
        .header(reqwest::header::USER_AGENT, "File-Transfer-Client")
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .header("Sender-Email", sender_email) // Set the Sender-Email header directly
        .body(compressed_data)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Receiver has accepted the file.");
    } else {
        println!("Receiver declined or encountered an error.");
    }

    Ok(())
}

async fn receive_and_decompress_data(mut socket: tokio::net::TcpStream) -> Result<(), io::Error> {
    let mut size_buffer = [0; 4]; // Assuming the size is a 32-bit unsigned integer (4 bytes)

    socket.read_exact(&mut size_buffer).await?; // Read the size of the incoming data
    let total_bytes = u32::from_le_bytes(size_buffer).try_into().unwrap(); // Convert bytes to u32

    let mut received_bytes = 0;
    let mut decompressed_data = Vec::with_capacity(total_bytes);

    while received_bytes < total_bytes {
        let mut buffer = [0; 1024];
        let n = socket.read(&mut buffer).await?;
        decompressed_data.extend_from_slice(&buffer[..n]);
        received_bytes += n;

        let percent_received = (received_bytes as f64 / total_bytes as f64) * 100.0;
        println!("Received & Decompressed: {:.2}%", percent_received);
    }

    // Process decompressed_data as needed
    println!("Received & Decompressed Data: {:?}", decompressed_data);

    Ok(())
}

async fn notify_user() -> bool {
    println!("A file is being sent. Do you want to accept? (yes/no)");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().eq_ignore_ascii_case("yes")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("File Transfer Program")
        .version("1.0")
        .author("Your Name")
        .about("Compresses and sends a file to a receiver")
        .arg(
            Arg::new("file")
                .short('f')
                .long("file")
                .value_name("FILE")
                .help("Sets the input file to compress")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("receiver")
                .short('r')
                .long("receiver")
                .value_name("RECEIVER_URL")
                .help("Sets the receiver URL")
                .required(true)
                .num_args(1),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("RECEIVER_PORT")
                .help("Sets the receiver port")
                .num_args(1),
        )
        .get_matches();

    let file_path = matches.get_one::<String>("file").unwrap();
    let receiver_url = matches.get_one::<String>("receiver").unwrap();
    let receiver_port = match matches.get_one::<String>("port") {
        Some(port_str) => port_str.parse::<u16>().unwrap_or(8019), // Provide a default port if not provided
        None => 8019,
    };

    let sender_email = "sender@example.com"; // Replace with the sender's email address
    let compressed_data = compress_file(file_path).await?;
    send_compressed_data(compressed_data.clone(), receiver_url, sender_email).await?;

    // Establish TCP connection
    let receiver_address = format!("127.0.0.1:{}", receiver_port);
    let socket = tokio::net::TcpStream::connect(receiver_address).await?;
    


    // User notification and receiving logic
    if notify_user().await {
        receive_and_decompress_data(socket).await?;
    } else {
        println!("File transfer declined by user.");
    }

    Ok(())
}
