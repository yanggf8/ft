use worker::*;
#[event(fetch)]
async fn fetch(_req: Request, _env: Env, _ctx: Context) -> Result<Response> {
    Response::ok("ft-worker Phase A scaffold — ziwei engine via ft-ziwei")
}
