tonic::include_proto!("authentication");
use crate::config::storage::sign::profile;
use crate::types::{
    error::Result,
    id::Id,
    phone::Phone,
    token::Token,
    user::User,
    verification::{Code, Verification},
};
pub use authentication_server::AuthenticationServer;
use std::future::Future;
use tonic::{Request, Response, Status};

pub trait Authentication: Sync + Send + 'static + Sized {
    type Config: Sync + Send + 'static;
    fn new(config: Self::Config) -> AuthenticationServer<Self>;
    fn login(&self, phone: Phone) -> impl Future<Output = Result<Verification>> + Send;
    fn verify(&self, id: Id, code: Code) -> impl Future<Output = Result<Verified>> + Send;
    fn setup(&self, token: Token, name: String) -> Result<Authenticated>;
    fn refresh(&self, token: Token) -> Result<Authenticated>;
    fn change_phone(
        &self,
        token: Token,
        phone: Phone,
    ) -> impl Future<Output = Result<Verification>> + Send;
    fn confirm_change_phone(
        &self,
        token: Token,
        id: Id,
        code: Code,
    ) -> impl Future<Output = Result<Authenticated>> + Send;
}

#[tonic::async_trait]
impl<T: Authentication> authentication_server::Authentication for T {
    async fn login(
        &self,
        request: Request<Login>,
    ) -> Result<Response<crate::proto::types::verification::Verification>, Status> {
        let phone = request.into_inner().phone.parse()?;
        let verification = self.login(phone).await?;
        Ok(Response::new(verification.into()))
    }

    async fn verify(&self, request: Request<Verify>) -> Result<Response<Verified>, Status> {
        let Verify { id, code } = request.into_inner();
        let id = id.parse()?;
        let code = code.parse()?;
        let verified = self.verify(id, code).await?;
        Ok(Response::new(verified))
    }

    async fn setup(&self, request: Request<Setup>) -> Result<Response<Authenticated>, Status> {
        let Setup { token, name } = request.into_inner();
        let token = token.parse()?;
        let authenticated = self.setup(token, name)?;
        Ok(Response::new(authenticated))
    }

    async fn refresh(&self, request: Request<Refresh>) -> Result<Response<Authenticated>, Status> {
        let Refresh { refresh_token } = request.into_inner();
        let token = refresh_token.parse()?;
        let authenticated = self.refresh(token)?;
        Ok(Response::new(authenticated))
    }

    async fn change_phone(
        &self,
        req: Request<ChangePhone>,
    ) -> Result<Response<crate::proto::types::verification::Verification>, Status> {
        let ChangePhone { token, phone } = req.into_inner();
        let (token, phone) = (token.parse()?, phone.parse()?);
        let verification = self.change_phone(token, phone).await?.into();
        Ok(Response::new(verification))
    }

    async fn confirm_change_phone(
        &self,
        req: Request<ConfirmChangePhone>,
    ) -> Result<Response<Authenticated>, Status> {
        let ConfirmChangePhone { token, id, code } = req.into_inner();
        let (token, id, code) = (token.parse()?, id.parse()?, code.parse()?);
        let authenticated = self.confirm_change_phone(token, id, code).await?;
        Ok(Response::new(authenticated))
    }
}

impl Registered {
    pub fn new(id: Id, phone: Phone) -> Result<Self> {
        let token = Token::setup(id, phone);
        let token = token.tokenize()?;
        Ok(Registered { token })
    }
}

impl Authenticated {
    pub fn new(user: User) -> Result<Self> {
        let access_token = Token::access(user.id, user.phone).tokenize()?;
        let refresh_token = Token::refresh(user.id, user.phone).tokenize()?;

        let profile = profile(&user.id, None, true);
        let user = Some(user.into());
        Ok(Authenticated {
            access_token,
            refresh_token,
            user,
            profile,
        })
    }
}

impl Verified {
    pub fn authenticated(user: User) -> Result<Self> {
        let verified = Some(verified::Verified::Authenticated(Authenticated::new(user)?));
        Ok(Verified { verified })
    }

    pub fn registered(phone: Phone) -> Result<Self> {
        let id = Default::default();
        let verified = Some(verified::Verified::Registered(Registered::new(id, phone)?));
        Ok(Verified { verified })
    }
}
