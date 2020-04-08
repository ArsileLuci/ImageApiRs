use actix_web::{get, post, web, App, HttpServer, Responder,HttpResponse}; 
use actix_multipart::Multipart;

extern crate futures;
use futures::{StreamExt, TryStreamExt};

extern crate base64;
use base64::decode;

#[macro_use]
extern crate diesel;
extern crate dotenv;

extern crate uuid;

use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};

mod schema;

mod dtos;
use dtos::*;
mod actions;
mod models;

type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[get("/preview/{id}")]
async fn get_preview(
    pool: web::Data<DbPool>,
    info: web::Path<uuid::Uuid>,
) -> impl Responder {
    let id = info.into_inner();
    let conn = pool.get().expect("couldn't get db connection from pool");
    let img = actions::find_image_by_id(id, &conn);
    match img {
        Ok(opt)=>{
            match opt {
                Some(_image) => {
                    return HttpResponse::Ok().body(actions::make_preview(&_image.Content).unwrap());
                }
                None => {
                    return HttpResponse::NotFound().finish();
                }
            }
        }
        Err(_) =>{
            return HttpResponse::InternalServerError().finish();
        }
    }
}


#[get("/image/{id}")]
async fn get_image(
    pool: web::Data<DbPool>,
    info: web::Path<uuid::Uuid>,
) -> impl Responder {
    let id = info.into_inner();
    let conn = pool.get().expect("couldn't get db connection from pool");
    let img = actions::find_image_by_id(id, &conn);
    match img {
        Ok(opt)=>{
            match opt {
                Some(_image) => {
                    return HttpResponse::Ok().body(_image.Content);
                }
                None => {
                    return HttpResponse::NotFound().finish();
                }
            }
        }
        Err(_) =>{
            return HttpResponse::InternalServerError().finish();
        }
    } 
}

#[post("/addmultipart")]
async fn add_multipart(
    pool: web::Data<DbPool>,
    mut model: Multipart,
) -> impl Responder {
    let mut items : Vec<Vec<u8>> = Vec::new(); 
    while let Ok(Some(mut field)) = model.try_next().await {
        let mut bytes : Vec<u8> = Vec::new();
        while let Some(chunk) = field.next().await {
            bytes.append(&mut Vec::from(&chunk.unwrap()[0..]));
        }
        items.push(bytes);
    }
    let conn = pool.get().expect("couldn't get db connection from pool");
    let result = actions::insert_many(items, &conn);
    match result {
        Ok(values) => {
            return HttpResponse::Ok().json(values);
        }
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    }
}

#[post("/addbase64")]
async fn add_base64(
    pool: web::Data<DbPool>,
    model: web::Json<Base64Dto>,
) -> impl Responder {
    match model.Image.split(",").last() { //Пропускаем заголовок base64
        Some(base64) => {
            match decode(base64) {
                Ok(bytes) => {
                    let conn = pool.get().expect("couldn't get db connection from pool");
                    let result = actions::insert_image(bytes, &conn);
                    
                    match result {
                        Ok(id) => {
                            match id {
                                Some(value) => {
                                    return HttpResponse::Ok().json(ResponseDto {Id:value});
                                }
                                None => {
                                    println!("bad image");
                                    return HttpResponse::BadRequest().body("image provided in not supported format, supported formats png, bmp, tiff, jpeg");
                                }
                            }
                        }
                        Err(_) => {
                            return HttpResponse::InternalServerError().finish();
                        }
                    }
                }
                Err(_) => {
                    println!("bruh");
                    return HttpResponse::BadRequest().body("base64 incorrectly encoded");
                }
            }
        }
        None => {
            return HttpResponse::BadRequest().body("base64 encoded string was not presented");
        }
    }
}

#[post("/addurl")]
async fn add_url(
    pool: web::Data<DbPool>,
    model: web::Json<UrlDto>,
) -> impl Responder {
    let bytes: Vec<u8>;
    let request = reqwest::get(&model.Url).await;
    match request {
        Ok(response) => {
            let response_bytes = response.bytes().await;
            match response_bytes {
                Ok(raw_bytes) => {
                    bytes = Vec::from(&raw_bytes[0..]);
                }
                Err(_) => {
                    return HttpResponse::BadRequest().body("error occured during recieving of image's bytestream");
                }
            }
        }
        Err(_) => {
            return HttpResponse::BadRequest().body("cannot get image url from the given url");
        }
    }
    let conn = pool.get().expect("couldn't get db connection from pool");
    let response_img = actions::insert_image(bytes, &conn);
    match response_img {
        Ok(id) => {
            match id {
                Some(value) => {
                    return HttpResponse::Ok().json(ResponseDto {Id:value});
                }
                None => {
                    return HttpResponse::BadRequest().body("image provided in not supported format, supported formats png, bmp, tiff, jpeg");
                }
            }
        }
        Err(_) => {
            return HttpResponse::InternalServerError().finish();
        }
    }

}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<SqliteConnection>::new(connspec);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");



    HttpServer::new(move || 
        App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(add_base64)
            .service(get_preview)
            .service(get_image)
            .service(add_url)
            .service(add_multipart)
        )
        .bind("127.0.0.1:8080")?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test;
    use std::io::{Read};
    #[actix_rt::test]
    async fn test_addurl_and_getimage_ok() {

        dotenv::dotenv().ok();

        let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let manager = ConnectionManager::<SqliteConnection>::new(connspec);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let mut app = test::init_service(App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(get_image)
            .service(add_url)
        ).await;
        let dto = UrlDto {Url: "https://media.discordapp.net/attachments/617725195691491349/692406625381646496/yin14x1vbuo41.png".to_string()};
        let add_request = test::TestRequest::post()
            .uri("/addurl")
            .set_json(&dto)
            .to_request();
        let result: ResponseDto = test::read_response_json(&mut app, add_request).await;
        let route = format!("/image/{}",result.Id);
        let get_request = test::TestRequest::get()
            .uri(&route)
            .to_request();
        let body = test::read_response(&mut app, get_request)
            .await;
        let bytes = reqwest::get(&dto.Url)
            .await
            .expect("download link is broken")
            .bytes()
            .await
            .expect("error during image downloading");
        
        assert_eq!(bytes, body);
    }

    #[actix_rt::test]
    async fn test_addbase64_png_and_preview_ok() {

        dotenv::dotenv().ok();

        let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let manager = ConnectionManager::<SqliteConnection>::new(connspec);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let mut app = test::init_service(App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(get_preview)
            .service(add_base64)
        ).await;
        
        let mut file = std::fs::OpenOptions::new().read(true).open("testbase64.png").expect("missing testbase64.png");
        let mut bytes = Vec::<u8>::new();
        file.read_to_end(&mut bytes).expect("somethink gone wrong during reading testbase64.png");

        let base64string = base64::encode(bytes);
        //println!("{}", base64string);
        let dto = Base64Dto { Image: base64string };
        let add_request = test::TestRequest::post()
            .uri("/addbase64")
            .set_json(&dto)
            .to_request();
        let result: ResponseDto = test::read_response_json(&mut app, add_request).await;
        let route = format!("/preview/{}",result.Id);
        let get_request = test::TestRequest::get()
            .uri(&route)
            .to_request();
        let body = test::call_service(&mut app, get_request)
            .await;
        
        assert!(body.status().is_success());
    }

    #[actix_rt::test]
    async fn test_addbase64_notimage_is_client_error() {

        dotenv::dotenv().ok();

        let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let manager = ConnectionManager::<SqliteConnection>::new(connspec);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let mut app = test::init_service(App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(add_base64)
        ).await;
        
        let mut file = std::fs::OpenOptions::new().read(true).open("notimage.txt").expect("missing notimage.txt");
        let mut bytes = Vec::<u8>::new();
        file.read_to_end(&mut bytes).expect("somethink gone wrong during reading notimage.txt");

        let base64string = base64::encode(bytes);
        //println!("{}", base64string);
        let dto = Base64Dto { Image: base64string };
        let add_request = test::TestRequest::post()
            .uri("/addbase64")
            .set_json(&dto)
            .to_request();
        let result = test::call_service(&mut app, add_request)
            .await;
        assert!(result.status().is_client_error());
    }

    #[actix_rt::test]
    async fn test_get_image_with_bad_id_is_client_error() {

        dotenv::dotenv().ok();

        let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let manager = ConnectionManager::<SqliteConnection>::new(connspec);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let mut app = test::init_service(App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(get_image)
        ).await;

        let get_request = test::TestRequest::get()
            .uri("/image/0000-0000-0000-0000")
            .to_request();
        let result = test::call_service(&mut app, get_request)
            .await;
        assert!(result.status().is_client_error());
    }
    
    #[actix_rt::test]
    async fn test_get_preview_with_bad_id_is_client_error() {

        dotenv::dotenv().ok();

        let connspec = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let manager = ConnectionManager::<SqliteConnection>::new(connspec);
        let pool = r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.");

        let mut app = test::init_service(App::new()
            .data(pool.clone())
            .app_data(web::PayloadConfig::new(32 * 1024 * 1024 * 1024))
            .app_data(web::JsonConfig::default().limit(1024 * 1024 * 32 * 1024))
            .service(get_preview)
        ).await;
        
        let get_request = test::TestRequest::get()
            .uri("/preview/0000-0000-0000-0000")
            .to_request();
        let result = test::call_service(&mut app, get_request)
            .await;
        assert!(result.status().is_client_error());
    }   

}