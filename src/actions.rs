use diesel::prelude::*;
use uuid::Uuid;
use crate::models;

use image::imageops;

pub fn find_image_by_id(
    search_id: Uuid,
    conn: &SqliteConnection,
) -> Result<Option<models::Image>, diesel::result::Error> {
    use crate::schema::images::dsl::*;

    let img = images
        .filter(Id.eq(search_id.to_string()))
        .first::<models::Image>(conn)
        .optional()?;

    Ok(img)
}

pub fn insert_image(
    data:Vec<u8>,
    conn: &SqliteConnection,
) -> Result<Option<String>, diesel::result::Error> {
    use crate::schema::images::dsl::*;

    if !validate_image(&data[0..]) {
        return Ok(None);
    }

    let new_image = models::Image {
        Content : data,
        Id: Uuid::new_v4().to_string(),
    };

    diesel::insert_into(images).values(&new_image).execute(conn)?;
    //println!("{:?}", new_image);
    Ok(Some(new_image.Id))
}

pub fn insert_many(
    items:Vec<Vec<u8>>,
    conn: &SqliteConnection,
) -> Result<Vec<Option<String>>, diesel::result::Error> {
    use crate::schema::images::dsl::*;

    let mut imgs : Vec<Option<String>> = Vec::new();
    for image in items {
        
        if !validate_image(&image[0..]) {
            imgs.push(None);
        }
        else {    
            let new_image = models::Image {
                Content : image,
                Id: Uuid::new_v4().to_string(),
            };
            diesel::insert_into(images).values(&new_image).execute(conn)?;
            //println!("{:?}", new_image);
            imgs.push(Some(new_image.Id));
        } 
    }

    return Ok(imgs);
}


//Проаеряет изображение на соответствие одному из форматов BMP, GIF, JPEG, PNG, TIFF при помощи проверки заголовка файла
fn validate_image(img_ref:&[u8] ) -> bool {
    if img_ref.starts_with(&[137, 80, 78, 71]) { //PNG
        return true;
    }
    if img_ref.starts_with(&[73, 73, 42 ]) { //TIFF
        return true;
    }
    if img_ref.starts_with(&[77, 77, 42 ]) { //TIFF
        return true;
    }
    if img_ref.starts_with(&[255, 216, 255, 224 ]) { //JPEG
        return true;
    }
    if img_ref.starts_with(&[255, 216, 255, 225 ]) { //JPEG canon
        return  true;
    }
    if img_ref.starts_with("BM".to_ascii_uppercase().as_bytes()) { //BMP
        return  true;
    }
    if img_ref.starts_with("GIF".to_ascii_uppercase().as_bytes()) { //GIF
        return  true;
    }
    return false;
}

pub fn make_preview(img: &[u8]) -> Option<Vec<u8>> {
    match image::load_from_memory(img) {
        Ok(_image) => {
            let mut result = Vec::new();
            let mut jpeg_encoder = image::jpeg::JPEGEncoder::new(&mut result);
            jpeg_encoder.encode(&_image.resize_to_fill(100, 100, imageops::FilterType::Triangle).to_bytes(), 100, 100, image::ColorType::Rgb8).unwrap();
            return Some(result);
        }
        Err(_)=> {
            return None;
        }
    }
}