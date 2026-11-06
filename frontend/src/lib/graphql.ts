export const GRAPHQL_ENDPOINT =
  typeof window !== 'undefined'
    ? (window.location.port === '4321' ? 'http://127.0.0.1:8000/graphql' : '/graphql')
    : 'http://127.0.0.1:8000/graphql';

export interface GqlUser {
  id: string;
  name: string;
  email: string;
  role: string;
  createdAt?: string;
}

export interface GqlLoginResponse {
  login: {
    token: string;
    tokenType: string;
    user: GqlUser;
  };
}

export interface GqlUsersResponse {
  users: GqlUser[];
}

export interface GqlMeResponse {
  me: GqlUser | null;
}

export interface CreateUserInput {
  name: string;
  email: string;
  password?: string;
  role?: string;
}

export interface UpdateUserInput {
  name?: string;
  email?: string;
  password?: string;
  role?: string;
}

export interface GqlCreateUserResponse {
  createUser: GqlUser;
}

export interface GqlUpdateUserResponse {
  updateUser: GqlUser;
}

export interface GqlDeleteUserResponse {
  deleteUser: boolean;
}

// ---------------------------------------------------------------------------
// GraphQL Queries & Mutations
// ---------------------------------------------------------------------------

export const USERS_QUERY = `
  query GetUsers {
    users {
      id
      name
      email
      role
      createdAt
    }
  }
`;

export const ME_QUERY = `
  query GetMe {
    me {
      id
      name
      email
      role
      createdAt
    }
  }
`;

export const CREATE_USER_MUTATION = `
  mutation CreateUser($input: CreateUserGqlInput!) {
    createUser(input: $input) {
      id
      name
      email
      role
      createdAt
    }
  }
`;

export const UPDATE_USER_MUTATION = `
  mutation UpdateUser($id: String!, $input: UpdateUserGqlInput!) {
    updateUser(id: $id, input: $input) {
      id
      name
      email
      role
      createdAt
    }
  }
`;

export const DELETE_USER_MUTATION = `
  mutation DeleteUser($id: String!) {
    deleteUser(id: $id)
  }
`;

// ---------------------------------------------------------------------------
// Network Request & Auth Helpers
// ---------------------------------------------------------------------------

export async function graphqlRequest<T = any>(
  query: string,
  variables: Record<string, any> = {},
  token?: string | null
): Promise<T> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  const authToken = token !== undefined ? token : getAuthToken();
  if (authToken) {
    headers['Authorization'] = `Bearer ${authToken}`;
  }

  const res = await fetch(GRAPHQL_ENDPOINT, {
    method: 'POST',
    headers,
    body: JSON.stringify({ query, variables }),
  });

  const json = await res.json();
  if (json.errors && json.errors.length > 0) {
    throw new Error(json.errors[0].message || 'Terjadi kesalahan pada GraphQL');
  }

  return json.data;
}

export function getAuthToken(): string | null {
  if (typeof window === 'undefined') return null;
  return localStorage.getItem('auth_token');
}

export function setAuthSession(token: string, user: GqlUser) {
  if (typeof window === 'undefined') return;
  localStorage.setItem('auth_token', token);
  localStorage.setItem('auth_user', JSON.stringify(user));
}

export function clearAuthSession() {
  if (typeof window === 'undefined') return;
  localStorage.removeItem('auth_token');
  localStorage.removeItem('auth_user');
}

export function getAuthUser(): GqlUser | null {
  if (typeof window === 'undefined') return null;
  const user = localStorage.getItem('auth_user');
  try {
    return user ? JSON.parse(user) : null;
  } catch {
    return null;
  }
}
