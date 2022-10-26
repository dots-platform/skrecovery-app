use signing::gg20_sm_manager::*;
use anyhow::{Result, Context};
use structopt::StructOpt;


use futures::Stream;
use rocket::data::ToByteUnit;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::stream::{stream, Event, EventStream};
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use tokio::sync::{Notify, RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let figment = rocket::Config::figment().merge((
        "limits",
        rocket::data::Limits::new().limit("string", 100.megabytes()),
    ));
    rocket::custom(figment)
        .mount("/", rocket::routes![subscribe, issue_idx, broadcast])
        .manage(Db::empty())
        .launch()
        .await?;
    Ok(())
}