use super::schema::images;

#[derive(Queryable, Insertable,Debug)]
pub struct Image {
    pub Id: String,
    pub Content: Vec<u8>
}