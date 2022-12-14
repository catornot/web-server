use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;
use server::ThreadPool;
use base64::decode;

#[derive(Serialize, Deserialize)]
struct AllowedResponse {
    get: Vec<String>,
}
struct Response {
    response: String,
    content: Vec<u8>,
}

impl Response {
    // fn set_not_found_reponse( &mut self ) {
    //     self.response = String::from( "HTTP/1.1 404 NOT FOUND" );
    //     self.content = fs::read_to_string( "404.html" ).unwrap();
    // }

    fn set_html_content(&mut self, request: &str) {
        println!("page {}", request);

        if request == "/" {
            self.content = fs::read("web_src/index.html").unwrap()
        } else {
            let mut path = request.to_string();
            path.push_str(".html");

            self.set_content_by_path(&path[..])
        }
    }

    fn set_content_by_path(&mut self, path: &str) {
        let mut path_str = path.to_string();

        if path_str.ends_with(".ico") {
            path_str.insert_str(0, "img/");
        }

        path_str.insert_str(0, "web_src/");

        self.content = match fs::read(path_str) {
            Err(err) => {
                println!("{}", err);
                println!("path : {}", path);
                let r = not_found_reponse();
                self.response = r.response;
                r.content
            }
            Ok(content) => content,
        }
    }
}

const OK: &str = "HTTP/1.1 200 OK";

fn main() {
    let listener: TcpListener = match TcpListener::bind("localhost:7878") { // "127.0.0.1:7878"
        Err(err) => {
            println!("port isn't available {}", err);
            return;
        }
        Ok(listener) => listener,
    };

    let pool = ThreadPool::new( 20 );

    for stream in listener.incoming() {
        let stream = stream.expect("couldn't connect");

        pool.execute( || {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024 * 4];

    stream.read(&mut buffer).expect("couldn't read stream");

    let mut response_ = get_response_from_request(buffer);

    let response_str = format!(
        "{}\r\nContent-Length: {}\r\n\r\n",
        response_.response,
        response_.content.len()
    );

    let mut response = Vec::from(response_str.as_bytes());
    response.append(&mut response_.content);

    stream.write(&response[..]).unwrap();
    stream.flush().unwrap();
}

fn get_response_from_request(buffer: [u8; 1024 * 4]) -> Response {
    let http_request: Vec<_> = buffer
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    let request = &http_request[0];
    println!("{}", request);

    let json = fs::read_to_string("allowed_requests.json").unwrap();

    let res = serde_json::from_str(&json[..]);

    let allowed: AllowedResponse = match res {
        Ok( allowed ) => allowed,
        Err( _ ) => return not_found_reponse()
    };

    let mut offset = 0;
    let mut request_type = "none";
    for c in request.chars() {
        if c == ' ' {
            request_type = &request[..offset];
            break;
        }
        offset += 1
    }

    let mut response: Response = not_found_reponse();

    if request_type == "GET" {
        // we may be able to make a function to handle post/del etc
        let allowed = &allowed.get;

        for a in request.split(" ") {
            for x in 0..allowed.len() {
                if allowed[x] == a {
                    println!("requested: {}", a);

                    response.response = OK.to_string();

                    if a.to_string().find(".") == None {
                        response.set_html_content(a);
                    } else {
                        response.set_content_by_path(a);
                    }
                }
            }
        }
    }
    else if request_type == "POST-IMAGE" {
        // println!("Request: {:#?}", http_request);

        response.response = OK.to_string();
        response.content = Vec::new();

        let mut contentlen = 0;
        let mut name = "ohno".to_string();
        

        for arg in &http_request {
            if arg.starts_with( "length" ) {
                contentlen = arg[8..].parse::<usize>().unwrap();
            }
            if arg.starts_with( "file-name" ) {
                let arg = arg.split_whitespace();
                // for a in arg {
                //     if a.starts_with( "file-name" ) { continue };

                //     name = a.to_string();
                // }

                name = arg
                    .into_iter()
                    .filter( | a | !a.starts_with( "file-name" ) )
                    .collect();
            }
        }

        if contentlen == 0 { return response; } // yes
        
        let body:String = http_request
        .into_iter()
        .filter( | a | a.starts_with( "body" ) )
        .collect();

        let split_body = body.split_whitespace();

        name = format!( "imgs/{}.png",name );

        let a:String = split_body
            .into_iter()
            .filter( | a | !a.starts_with( "body" ) )
            .collect();

        let a_buffer = decode( a ).unwrap();

        let mut file: File = match OpenOptions::new()
            .write(true)
            .append(true)
            .open(&name) {
                Ok( file ) => file,
                Err( _ ) => File::create( &name ).unwrap()
        };

        if let Err(e) = file.write_all( &a_buffer[..] ) {
            eprintln!("Couldn't write to file: {}", e);
        }

        println!( "content-length: {}", contentlen );
        println!( "file-name: {}", name );
    }

    return response;
}

fn not_found_reponse() -> Response {
    return Response {
        response: String::from("HTTP/1.1 404 NOT FOUND"),
        content: fs::read("web_src/404.html").unwrap(),
    };
}
