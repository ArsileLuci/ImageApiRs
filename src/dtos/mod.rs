extern crate serde;
use serde::{Serialize, Deserialize};
#[derive(Deserialize, Serialize)]
pub struct Base64Dto {
    pub Image : String
}
#[derive(Deserialize, Serialize)]
pub struct UrlDto {
    pub Url : String 
}
#[derive(Serialize, Deserialize)]
pub struct ResponseDto {
    pub Id : String,
}

