use rustls_pemfile::{read_all, Item};
use serde::Deserialize;
use std::{error::Error, fs, io};

#[derive(Clone, Deserialize, Debug)]
pub enum ClientCertAuth {
    PEMFile(String), // DERFile{cert_chain: String, key_file: String}
}

impl ClientCertAuth {
    pub fn get_client_cert(
        &self,
    ) -> Result<(Vec<rustls::Certificate>, rustls::PrivateKey), Box<dyn Error>> {
        match self {
            ClientCertAuth::PEMFile(filename) => {
                let f = fs::File::open(filename)?;
                let mut f = io::BufReader::new(f);

                let mut cert_chain: Vec<rustls::Certificate> = Vec::new();
                let mut privkey: Option<rustls::PrivateKey> = None;

                for item in read_all(&mut f)? {
                    match item {
                        Item::X509Certificate(a) => {
                            cert_chain.push(rustls::Certificate(a));
                        }
                        Item::PKCS8Key(a) | Item::RSAKey(a) => {
                            if privkey.is_none() {
                                privkey = Some(rustls::PrivateKey(a));
                            } else {
                                return Err(Box::new(io::Error::new(
                                    io::ErrorKind::Other,
                                    "Multiple private keys.",
                                )));
                            }
                        }
                    }
                }

                match privkey {
                    Some(a) => Ok((cert_chain, a)),
                    None => Err(Box::new(io::Error::new(
                        io::ErrorKind::Other,
                        "Missing private key.",
                    ))),
                }
            }
        }
    }
}
