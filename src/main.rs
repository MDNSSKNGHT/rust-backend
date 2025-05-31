#[macro_use]
extern crate diesel;

pub mod schema;
pub mod models;

use actix_web::error::BlockingError;
use tera::Tera;
use dotenv::dotenv;
use std::env;

use diesel::prelude::*;
use diesel::pg::PgConnection;

use diesel::r2d2::{self, ConnectionManager};
use diesel::r2d2::Pool;

use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};

use self::models::{Post, NewPost, NewPostHandler};
use self::schema::posts;
use self::schema::posts::dsl::*;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[get("/")]
async fn index(
    pool: web::Data<DbPool>,
    template_manager: web::Data<tera::Tera>
) -> impl Responder {
    let mut connection = pool.get().expect("Failed to retrieve database");

    match web::block(move || {posts.load::<Post>(&mut connection)}).await {
        Ok(data) => {
            let data = data.unwrap();

            let mut ctx = tera::Context::new();
            ctx.insert("posts", &data);

            HttpResponse::Ok()
                .content_type("text/html")
                .body(template_manager.render("index.html", &ctx).unwrap())
        }
        Err(_err) => HttpResponse::Ok().body("Error while receiving data"),
    }
}

#[get("/blog/{blog_slug}")]
async fn get_post(
    pool: web::Data<DbPool>,
    template_manager: web::Data<tera::Tera>,
    blog_slug: web::Path<String>
) -> impl Responder {
    let mut connection = pool.get().expect("Failed to retrieve database");

    match web::block(move || {
        posts.filter(slug.eq(blog_slug.into_inner()))
            .load::<Post>(&mut connection)
    }).await {
        Ok(data) => {
            let data = data.unwrap();

            if data.len() == 0 {
                return HttpResponse::NotFound().finish();
            }

            let data = &data[0];

            let mut ctx = tera::Context::new();
            ctx.insert("post", data);

            HttpResponse::Ok()
                .content_type("text/html")
                .body(template_manager.render("posts.html", &ctx).unwrap())
        }
        Err(_err) => HttpResponse::Ok().body("Error while receiving data"),
    }
}

#[post("/new_post")]
async fn new_post(pool: web::Data<DbPool>,
    item: web::Json<NewPostHandler>) -> impl Responder {
    let mut connection = pool.get().expect("Failed to retrieve database");

    match web::block(move || {
        Post::create_post(&mut connection, &item)
    }).await {
        Ok(data) => {
            HttpResponse::Ok().body(format!("{:?}", data))
        }
        Err(_err) => HttpResponse::Ok().body("Error while receiving data"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("Database URL not found");
    let port: u16 = env::var("PORT").expect("Port not found")
        .parse()
        .unwrap();

    // let mut connection = PgConnection::establish(&database_url)
    //     .expect("Could not connect to database");
    //
    // let new_post = NewPost {
    //     title: "My first blogpost",
    //     body: "Lorem ipsum",
    //     slug: "first-post"
    // };
    //
    // diesel::insert_into(posts)
    //     .values(&new_post)
    //     .execute(&mut connection)
    //     .unwrap();

    let connection = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder().build(connection).expect("Failed to build Pool");

    HttpServer::new(move || {
        let tera = Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*"))
            .unwrap();

        App::new()
            .service(index)
            .service(new_post)
            .service(get_post)
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(tera))
    }).bind(("0.0.0.0", port)).unwrap().run().await
}
