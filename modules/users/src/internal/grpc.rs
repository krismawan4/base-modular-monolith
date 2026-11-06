use std::sync::Arc;
use async_trait::async_trait;
use tonic::{Request, Response, Status};

use crate::{
    CreateUserInput, DynUsersClient, UpdateUserInput, UserDto, UserError, UsersClient,
};

pub mod proto {
    tonic::include_proto!("users");
}

impl From<UserDto> for proto::UserProto {
    fn from(dto: UserDto) -> Self {
        Self {
            id: dto.id,
            name: dto.name,
            email: dto.email,
            role: dto.role,
            created_at: dto.created_at,
        }
    }
}

impl From<proto::UserProto> for UserDto {
    fn from(p: proto::UserProto) -> Self {
        Self {
            id: p.id,
            name: p.name,
            email: p.email,
            role: p.role,
            created_at: p.created_at,
        }
    }
}

fn error_to_status(err: UserError) -> Status {
    match err {
        UserError::NotFound => Status::not_found(err.to_string()),
        UserError::DuplicateEmail => Status::already_exists(err.to_string()),
        UserError::ValidationError(msg) => Status::invalid_argument(msg),
        UserError::Internal(msg) => Status::internal(msg),
    }
}

fn status_to_error(status: Status) -> UserError {
    match status.code() {
        tonic::Code::NotFound => UserError::NotFound,
        tonic::Code::AlreadyExists => UserError::DuplicateEmail,
        tonic::Code::InvalidArgument => UserError::ValidationError(status.message().to_string()),
        _ => UserError::Internal(status.message().to_string()),
    }
}

// ---------------------------------------------------------------------------
// gRPC Server Implementation (Runs on microservice side)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct UsersGrpcServer {
    client: DynUsersClient,
}

impl UsersGrpcServer {
    pub(crate) fn new(client: DynUsersClient) -> Self {
        Self { client }
    }
}

#[tonic::async_trait]
impl proto::users_grpc_service_server::UsersGrpcService for UsersGrpcServer {
    async fn create_user(
        &self,
        request: Request<proto::CreateUserRequest>,
    ) -> Result<Response<proto::CreateUserResponse>, Status> {
        let req = request.into_inner();
        let input = CreateUserInput {
            name: req.name,
            email: req.email,
            password: req.password,
            role: req.role,
        };

        let user = self
            .client
            .create_user(input)
            .await
            .map_err(error_to_status)?;

        Ok(Response::new(proto::CreateUserResponse {
            user: Some(user.into()),
        }))
    }

    async fn get_user_by_id(
        &self,
        request: Request<proto::GetUserByIdRequest>,
    ) -> Result<Response<proto::GetUserByIdResponse>, Status> {
        let req = request.into_inner();
        let user = self
            .client
            .get_user_by_id(&req.id)
            .await
            .map_err(error_to_status)?;

        Ok(Response::new(proto::GetUserByIdResponse {
            user: Some(user.into()),
        }))
    }

    async fn list_users(
        &self,
        _request: Request<proto::ListUsersRequest>,
    ) -> Result<Response<proto::ListUsersResponse>, Status> {
        let users = self
            .client
            .list_users()
            .await
            .map_err(error_to_status)?;

        Ok(Response::new(proto::ListUsersResponse {
            users: users.into_iter().map(Into::into).collect(),
        }))
    }

    async fn update_user(
        &self,
        request: Request<proto::UpdateUserRequest>,
    ) -> Result<Response<proto::UpdateUserResponse>, Status> {
        let req = request.into_inner();
        let input = UpdateUserInput {
            name: req.name,
            email: req.email,
            password: req.password,
            role: req.role,
        };

        let user = self
            .client
            .update_user(&req.id, input)
            .await
            .map_err(error_to_status)?;

        Ok(Response::new(proto::UpdateUserResponse {
            user: Some(user.into()),
        }))
    }

    async fn delete_user(
        &self,
        request: Request<proto::DeleteUserRequest>,
    ) -> Result<Response<proto::DeleteUserResponse>, Status> {
        let req = request.into_inner();
        self.client
            .delete_user(&req.id)
            .await
            .map_err(error_to_status)?;

        Ok(Response::new(proto::DeleteUserResponse { success: true }))
    }
}

// ---------------------------------------------------------------------------
// gRPC Client Implementation (Implements trait UsersClient via gRPC)
// ---------------------------------------------------------------------------

pub(crate) struct UsersGrpcClient {
    client: proto::users_grpc_service_client::UsersGrpcServiceClient<tonic::transport::Channel>,
}

impl UsersGrpcClient {
    pub(crate) async fn connect(addr: String) -> Result<DynUsersClient, UserError> {
        let channel = tonic::transport::Endpoint::from_shared(addr)
            .map_err(|e| UserError::Internal(e.to_string()))?
            .connect()
            .await
            .map_err(|e| UserError::Internal(e.to_string()))?;

        let client = proto::users_grpc_service_client::UsersGrpcServiceClient::new(channel);
        Ok(Arc::new(Self { client }))
    }
}

#[async_trait]
impl UsersClient for UsersGrpcClient {
    async fn create_user(&self, input: CreateUserInput) -> Result<UserDto, UserError> {
        let mut client = self.client.clone();
        let req = proto::CreateUserRequest {
            name: input.name,
            email: input.email,
            password: input.password,
            role: input.role,
        };

        let res = client
            .create_user(req)
            .await
            .map_err(status_to_error)?;

        res.into_inner()
            .user
            .map(UserDto::from)
            .ok_or_else(|| UserError::Internal("Empty user response from gRPC service".to_string()))
    }

    async fn get_user_by_id(&self, id: &str) -> Result<UserDto, UserError> {
        let mut client = self.client.clone();
        let req = proto::GetUserByIdRequest {
            id: id.to_string(),
        };

        let res = client
            .get_user_by_id(req)
            .await
            .map_err(status_to_error)?;

        res.into_inner()
            .user
            .map(UserDto::from)
            .ok_or_else(|| UserError::Internal("Empty user response from gRPC service".to_string()))
    }

    async fn list_users(&self) -> Result<Vec<UserDto>, UserError> {
        let mut client = self.client.clone();
        let req = proto::ListUsersRequest {};

        let res = client
            .list_users(req)
            .await
            .map_err(status_to_error)?;

        Ok(res
            .into_inner()
            .users
            .into_iter()
            .map(UserDto::from)
            .collect())
    }

    async fn update_user(&self, id: &str, input: UpdateUserInput) -> Result<UserDto, UserError> {
        let mut client = self.client.clone();
        let req = proto::UpdateUserRequest {
            id: id.to_string(),
            name: input.name,
            email: input.email,
            password: input.password,
            role: input.role,
        };

        let res = client
            .update_user(req)
            .await
            .map_err(status_to_error)?;

        res.into_inner()
            .user
            .map(UserDto::from)
            .ok_or_else(|| UserError::Internal("Empty user response from gRPC service".to_string()))
    }

    async fn delete_user(&self, id: &str) -> Result<(), UserError> {
        let mut client = self.client.clone();
        let req = proto::DeleteUserRequest {
            id: id.to_string(),
        };

        client
            .delete_user(req)
            .await
            .map_err(status_to_error)?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "grpc_test.rs"]
mod tests;

