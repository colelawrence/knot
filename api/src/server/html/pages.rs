/*
pub fn index(req: &HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    use templates::*;
    let req_session = req.session();
    Box::new(
        access_guard(req).and_then(move |signin_state: SigninState| {
            let mut page = Page::default();
            req_session.apply_flash(&mut page)?;

            match signin_state {
                SigninState::Valid(ref auth) => page.person(&auth.person),
                SigninState::SignedOutByThirdParty => {
                    page.info("You've been signed out by a third party.")
                }
                SigninState::NotSignedIn => {}
            };
            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(templates::HelloTemplate { page }.render().unwrap()))
        }),
    )
}

pub fn upload_example(req: &HttpRequest<State>) -> Box<Future<Item = HttpResponse, Error = Error>> {
    use templates::*;
    let req_session = req.session();
    Box::new(
        access_guard(req).and_then(move |signin_state: SigninState| {
            let mut page = Page::default();
            req_session.apply_flash(&mut page)?;

            match signin_state {
                SigninState::Valid(ref auth) => page.person(&auth.person),
                SigninState::SignedOutByThirdParty => {
                    page.info("You've been signed out by a third party.")
                }
                SigninState::NotSignedIn => {}
            };

            if page.user_opt.is_none() {
                return Ok(HttpResponse::Found().header("location", "/").finish());
            }

            Ok(HttpResponse::Ok()
                .header(http::header::CONTENT_TYPE, "text/html")
                .body(templates::UploadTemplate { page }.render().unwrap()))
        }),
    )
}
*/
