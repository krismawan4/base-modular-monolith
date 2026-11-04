use std::sync::Arc;
use async_graphql::{Context, EmptySubscription, InputObject, Object, Schema, SimpleObject};
use axum::{
    extract::Extension,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use users::DynUsersClient;

// ---------------------------------------------------------------------------
// 1. GraphQL Types & Inputs
// ---------------------------------------------------------------------------

#[derive(SimpleObject, Clone, Debug)]
pub struct UserType {
    pub id:         String,
    pub name:       String,
    pub email:      String,
    pub role:       String,
    pub created_at: String,
}

impl From<users::UserDto> for UserType {
    fn from(dto: users::UserDto) -> Self {
        Self {
            id:         dto.id,
            name:       dto.name,
            email:      dto.email,
            role:       dto.role,
            created_at: dto.created_at,
        }
    }
}

#[derive(InputObject)]
pub struct CreateUserGqlInput {
    pub name:     String,
    pub email:    String,
    pub password: Option<String>,
    pub role:     Option<String>,
}

// ---------------------------------------------------------------------------
// 2. Query Root
// ---------------------------------------------------------------------------

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Endpoint testing sederhana
    async fn hello(&self) -> &str {
        "Hello from Axum GraphQL!"
    }

    /// Mengambil semua user yang terdaftar
    async fn users(&self, ctx: &Context<'_>) -> async_graphql::Result<Vec<UserType>> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let list = users_client
            .list_users()
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(list.into_iter().map(UserType::from).collect())
    }

    /// Mengambil data user berdasarkan ID
    async fn user_by_id(
        &self,
        ctx: &Context<'_>,
        id: String,
    ) -> async_graphql::Result<Option<UserType>> {
        let users_client = ctx.data::<DynUsersClient>()?;
        match users_client.get_user_by_id(&id).await {
            Ok(dto) => Ok(Some(UserType::from(dto))),
            Err(users::UserError::NotFound) => Ok(None),
            Err(e) => Err(async_graphql::Error::new(e.to_string())),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Mutation Root
// ---------------------------------------------------------------------------

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Membuat user baru
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        input: CreateUserGqlInput,
    ) -> async_graphql::Result<UserType> {
        let users_client = ctx.data::<DynUsersClient>()?;
        let created = users_client
            .create_user(users::CreateUserInput {
                name:     input.name,
                email:    input.email,
                password: input.password,
                role:     input.role,
            })
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(UserType::from(created))
    }

    /// Menghapus user berdasarkan ID
    async fn delete_user(&self, ctx: &Context<'_>, id: String) -> async_graphql::Result<bool> {
        let users_client = ctx.data::<DynUsersClient>()?;
        users_client
            .delete_user(&id)
            .await
            .map_err(|e| async_graphql::Error::new(e.to_string()))?;

        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// 4. Schema & Router
// ---------------------------------------------------------------------------

pub type AppSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(users_client: DynUsersClient) -> AppSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(users_client)
        .finish()
}

pub async fn graphql_playground() -> impl IntoResponse {
    Html(
        async_graphql::http::GraphiQLSource::build()
            .endpoint("/graphql")
            .finish(),
    )
}

pub async fn graphql_handler(
    Extension(schema): Extension<Arc<AppSchema>>,
    Json(req): Json<async_graphql::Request>,
) -> Json<async_graphql::Response> {
    Json(schema.execute(req).await)
}

pub fn graphql_routes(users_client: DynUsersClient) -> Router {
    let schema = Arc::new(build_schema(users_client));
    Router::new()
        .route("/graphql", get(graphql_playground).post(graphql_handler))
        .layer(Extension(schema))
}

