mod function;

use function::*;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tonic::{transport::Server, Request, Response, Status};

struct NeonRef {
    channel: Channel,
    callback: Root<JsFunction>,
}

fn channel<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Channel> {
    static CHANNEL: OnceCell<Channel> = OnceCell::new();
    CHANNEL.get_or_try_init(|| Ok(cx.channel()))
}

static CHANNEL_CELL: OnceCell<Channel> = OnceCell::new();
static CALLBACK_CELL: OnceCell<Root<JsFunction>> = OnceCell::new();

fn start_func(mut cx: FunctionContext) -> JsResult<JsUndefined> {
    CHANNEL_CELL.set(cx.channel());
    CALLBACK_CELL.set(cx.argument::<JsFunction>(0)?.root(&mut cx));

    // let intercept = |mut req: Request<()>| -> Result<Request<()>, Status> {
    //     req.extensions_mut().insert(NEON_CELL.get());
    //     Ok(req)
    // };

    let addr = "[::1]:50051".parse().unwrap();
    let func = Function {};
    let svc = function_server::FunctionServer::new(func);

    let rt = runtime(&mut cx)?;
    let result = rt.block_on(Server::builder().add_service(svc).serve(addr));

    Ok(cx.undefined())
}

fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub struct Function {}

#[tonic::async_trait]
impl function_server::Function for Function {
    async fn process(
        &self,
        request: Request<FunctionRequest>,
    ) -> Result<Response<FunctionResponse>, Status> {
        println!("Got a request: {:?}", request);

        let value = request.into_inner().value;
        let channel: &Channel = CHANNEL_CELL.get().unwrap();

        // let (sender, receiver) = tokio::sync::oneshot::channel();

        channel.send(|mut cx| {
            let f: &Root<JsFunction> = CALLBACK_CELL.get().unwrap(); 
            let f = f.into_inner(&mut cx);
            // let this = cx.undefined();
            // let value: Handle<JsString> = f.call(&mut cx, this, [])?;
            // sender.send(value.value(&mut cx));

            Ok(())
        });

        //let value = receiver.recv().await.unwrap();
        let reply = FunctionResponse { value: value };

        Ok(Response::new(reply))
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("startFunc", start_func)?;
    Ok(())
}
