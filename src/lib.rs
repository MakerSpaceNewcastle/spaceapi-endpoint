use spaceapi::{Contact, Link, Location, State, StatusBuilder};
use worker::{event, Context, Env, Request, Response};

#[event(fetch)]
async fn main(_: Request, _: Env, _: Context) -> worker::Result<Response> {
    let status = StatusBuilder::v14("Maker Space")
        .logo("http://makerspace.pbworks.com/w/file/fetch/43988924/makerspace_logo.png")
        .url("https://www.makerspace.org.uk/")
        .location(Location {
            address: Some("Maker Space, c/o Orbis Community, Ground Floor, 65 High Street, Gateshead, NE8 2AP".into()),
            lat: 54.9652,
            lon: -1.60233,
            timezone: Some("Europe/London".into())
        })
    .contact(Contact {
        matrix: Some("#makerspace-ncl:matrix.org".into()),
        ml: Some("north-east-makers@googlegroups.com".into()),
        twitter: Some("@maker_space".into()),
        ..Default::default()
    })
    .add_link(Link {
        name: "Maker Space Wiki".into(),
        url: "http://makerspace.pbworks.com".into(),
        ..Default::default()
    })
    .add_link(Link {
        name: "North East Makers mailing list".into(),
        url: "https://groups.google.com/g/north-east-makers".into(),
        ..Default::default()
    })
    .add_project("https://github.com/MakerSpaceNewcastle")
    .state(State {
        open: Some(false),
        ..Default::default()
    })
    .build()
    .expect("basic space status should be created");

    Response::from_json(&status)
}
