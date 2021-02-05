use std::collections::HashMap;
use std::io::Write;

use actix_multipart::Multipart;
use actix_web::{error, web, App, Error, HttpResponse, HttpServer, HttpRequest};
use tera::Tera;
use serde::Deserialize;
use futures::StreamExt;
use rand::{thread_rng, Rng};

#[derive(Debug, Deserialize)]
struct UserData {
    name: String,
    gender: String
}

#[allow(dead_code)]
async fn get_local_address() -> String {
    use std::net::UdpSocket;
    let ip_str = UdpSocket::bind("0.0.0.0:8080").unwrap();
    &ip_str.connect("8.8.8.8:80").unwrap();
    let split_vec =  &ip_str.local_addr().unwrap().to_string();
    format!("{}", split_vec.split(":").collect::<Vec<&str>>()[0])
}

async fn gen_temp(tera_tmpl: web::Data<tera::Tera>, req: HttpRequest) -> Result<HttpResponse, Error> {
    let mut rng = thread_rng();
    let tmp: f32 = rng.gen_range(35.0, 36.9);
    let tmp_str = format!("{:.1}", tmp);
    let mut ctx = tera::Context::new();
    ctx.insert("temp_T", &tmp_str.to_owned());
    ctx.insert("addr_T", &req.head().peer_addr.unwrap().to_owned());
    ctx.insert("agent_T", &req.head().headers()
                        .get("user-agent").unwrap().to_str().unwrap().to_owned());
    let bod = tera_tmpl.render("tmp.html", &ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(bod.unwrap()))
}

async fn ps_process(formx: web::Form<UserData>) -> Result<HttpResponse, Error> {
    Ok(HttpResponse::Ok().content_type("text/html").body(format!("{:?}", formx)))
}

async fn index(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let s = if let Some(name) = query.get("name") {
        println!("Posted data from template: {:?}", query);
        let mut ctx = tera::Context::new();
        ctx.insert("name", &name.to_owned());
        ctx.insert("text", &"Welcome!".to_owned());
        tmpl.render("user.html", &ctx)
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    } else {
        tmpl.render("index.html", &tera::Context::new())
            .map_err(|_| error::ErrorInternalServerError("Template error"))?
    };
    Ok(HttpResponse::Ok().content_type("text/html").body(s))
}

async fn upload(tera_tmpl: web::Data<tera::Tera>) -> Result<HttpResponse, Error> {
    let ctx = tera::Context::new();
    let bdy = tera_tmpl.render("upload.html", &ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(bdy.unwrap()))
}

async fn save_file(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut f_name: String = String::new();
    while let Some(item) = payload.next().await {
        let mut field = item?;
        let content_type = field.content_disposition().unwrap();
        let filename = content_type.get_filename().unwrap();
        f_name = String::from(filename);
        let filepath = format!("./tmp/{}", filename);
        let mut f = web::block(|| std::fs::File::create(filepath))
            .await
            .unwrap();
        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f = web::block(move || f.write_all(&data).map(|_| f)).await?;
        }
    }
    
    match f_name.as_str() {
        "" => Ok(HttpResponse::Ok().content_type("text/plain").body("File upload failed.")),
        a_name @ _ => Ok(HttpResponse::Ok().content_type("text/plain").body(format!("{} Uploaded successful.", a_name)))
    }
}

async fn display_image(filename: web::Path<String>) -> Result<HttpResponse, Error> {
    let filepath = format!("./tmp/{}", filename);
    let img_data = web::block(|| {
        std::fs::read(filepath)
    })
        .await
        .unwrap();
    Ok(HttpResponse::Ok().content_type("image/jpeg").body(img_data))
}

async fn image_page(tera_tmpl: web::Data<tera::Tera>, imgname: web::Path<String>) -> Result<HttpResponse, Error> {
    let filepath = format!("/img/{}", imgname);
    let mut ctx = tera::Context::new();
    ctx.insert("f_path", filepath.as_str());
    let bdy = tera_tmpl.render("img.html", &ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(bdy.unwrap()))
}

async fn mp4_ret(videoname: web::Path<String>) -> Result<HttpResponse, Error> {
    let filepath = format!("./video/{}", videoname);
    let video_data = web::block(|| {
        std::fs::read(filepath)
    })
        .await
        .unwrap();
    Ok(HttpResponse::Ok().content_type("video/mpeg4").body(video_data))
}

async fn mp4_player(tera_tmpl: web::Data<tera::Tera>, videoname: web::Path<String>) -> Result<HttpResponse, Error> {
    let filepath = format!("/video/{}", videoname);
    let mut ctx = tera::Context::new();
    ctx.insert("mp4_pa", filepath.as_str());
    let bdy = tera_tmpl.render("video_player.html", &ctx);
    Ok(HttpResponse::Ok().content_type("text/html").body(bdy.unwrap()))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let port: u16 = 8000;
    println!("Host location address is: {}:{}", get_local_address().await, port);
    HttpServer::new(|| {
        let tera =
            Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/**/*")).unwrap();

        App::new()
            .data(tera)
            .service(web::resource("/").route(web::get().to(index)))
            .service(web::resource("/tmp").route(web::get().to(gen_temp)))
            .service(web::resource("/pos").route(web::post().to(ps_process)))
            .service(web::resource("/uplf").route(web::get().to(upload))
                                            .route(web::post().to(save_file)))
            .service(web::resource("/img/{filemane}").route(web::get().to(display_image)))
            .service(web::resource("/ipage/{imgname}").route(web::get().to(image_page)))
            .service(web::resource("/video/{videoname}").route(web::get().to(mp4_ret)))
            .service(web::resource("/player/{videoname}").route(web::get().to(mp4_player)))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}