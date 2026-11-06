'use client';

import { useState, useEffect } from 'react';
import {
  Users, ShieldCheck, Key, Activity,
  LayoutDashboard, Settings, LogOut, Menu, X, Loader2,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import {
  getAuthToken, getAuthUser, clearAuthSession, graphqlRequest,
  type GqlUser, type GqlUsersResponse, type GqlMeResponse,
} from '@/lib/graphql';

const USERS_QUERY = `
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

const ME_QUERY = `
  query GetMe {
    me {
      id
      name
      email
      role
    }
  }
`;

const NAV = [
  { label: 'Dashboard',   icon: LayoutDashboard, href: '/admin/' },
  { label: 'Users',       icon: Users,            href: '/admin/' },
  { label: 'Settings',    icon: Settings,         href: '/admin/' },
];

export default function AdminDashboard() {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [currentUser, setCurrentUser] = useState<GqlUser | null>(null);
  const [usersList, setUsersList] = useState<GqlUser[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = getAuthToken();
    if (!token) {
      window.location.href = '/login/';
      return;
    }

    // Set initial cached user
    const cached = getAuthUser();
    if (cached) setCurrentUser(cached);

    // Fetch live user and users list via GraphQL
    const loadData = async () => {
      try {
        const [meData, usersData] = await Promise.all([
          graphqlRequest<GqlMeResponse>(ME_QUERY, {}, token).catch(() => null),
          graphqlRequest<GqlUsersResponse>(USERS_QUERY, {}, token).catch(() => ({ users: [] })),
        ]);

        if (meData?.me) {
          setCurrentUser(meData.me);
        }
        if (usersData?.users) {
          setUsersList(usersData.users);
        }
      } catch (err) {
        console.error('Failed to load GraphQL data:', err);
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, []);

  const handleLogout = () => {
    clearAuthSession();
    window.location.href = '/login/';
  };

  const stats = [
    { label: 'Total User (Live GraphQL)', value: String(usersList.length), icon: Users, delta: 'Data real-time backend' },
    { label: 'User Role',                 value: currentUser?.role || 'Admin', icon: ShieldCheck, delta: 'Role terautentikasi' },
    { label: 'API Protokol',              value: 'GraphQL',  icon: Key, delta: 'POST /graphql' },
    { label: 'Status Sesi',               value: 'Aktif',    icon: Activity, delta: 'Bearer JWT Token' },
  ];

  return (
    <div className="flex h-screen bg-muted/30">
      {/* ── Sidebar ── */}
      <aside
        className={`
          fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r bg-white shadow-sm
          transition-transform duration-200
          ${sidebarOpen ? 'translate-x-0' : '-translate-x-full'}
          md:relative md:translate-x-0
        `}
      >
        {/* Logo */}
        <div className="flex h-16 items-center gap-2 border-b px-6">
          <span className="text-lg font-bold">⬡ ModularBase</span>
          <Badge variant="secondary" className="ml-auto text-[10px]">
            {currentUser?.role || 'Admin'}
          </Badge>
        </div>

        {/* Nav */}
        <nav className="flex-1 space-y-1 p-4">
          {NAV.map(({ label, icon: Icon, href }) => (
            <a
              key={label}
              href={href}
              className={`
                flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors
                ${label === 'Dashboard'
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'}
              `}
            >
              <Icon className="h-4 w-4" />
              {label}
            </a>
          ))}
        </nav>

        {/* Logout */}
        <div className="border-t p-4">
          <button
            onClick={handleLogout}
            className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-destructive hover:bg-destructive/10 transition-colors"
          >
            <LogOut className="h-4 w-4" />
            Keluar (Logout)
          </button>
        </div>
      </aside>

      {/* Overlay mobile */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/40 md:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* ── Main ── */}
      <div className="flex flex-1 flex-col overflow-hidden">
        {/* Topbar */}
        <header className="flex h-16 items-center border-b bg-white px-6 gap-4">
          <button
            className="md:hidden"
            onClick={() => setSidebarOpen(v => !v)}
          >
            {sidebarOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </button>
          <div>
            <h1 className="text-base font-semibold">Admin Panel</h1>
            <p className="text-xs text-muted-foreground">
              Terhubung ke GraphQL: <code>/graphql</code>
            </p>
          </div>
          <div className="ml-auto flex items-center gap-3">
            <Badge variant="default" className="hidden sm:inline-flex">
              GraphQL Bearer
            </Badge>
            <div className="flex items-center gap-2">
              <div className="h-8 w-8 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-xs font-bold uppercase">
                {currentUser?.name?.[0] || currentUser?.email?.[0] || 'U'}
              </div>
              <div className="hidden sm:block text-left">
                <div className="text-xs font-semibold leading-none">{currentUser?.name || 'User'}</div>
                <div className="text-[10px] text-muted-foreground">{currentUser?.email || ''}</div>
              </div>
            </div>
          </div>
        </header>

        {/* Content */}
        <main className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Stats Cards */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {stats.map(({ label, value, icon: Icon, delta }) => (
              <Card key={label}>
                <CardHeader className="flex flex-row items-center justify-between pb-2">
                  <CardTitle className="text-sm font-medium text-muted-foreground">
                    {label}
                  </CardTitle>
                  <Icon className="h-4 w-4 text-muted-foreground" />
                </CardHeader>
                <CardContent>
                  <div className="text-2xl font-bold">{value}</div>
                  <p className="mt-1 text-xs text-muted-foreground">{delta}</p>
                </CardContent>
              </Card>
            ))}
          </div>

          {/* User Table (Fetched via GraphQL) */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle>Daftar Pengguna (Live GraphQL)</CardTitle>
                <CardDescription>
                  Data diambil secara dinamis melalui GraphQL Query <code>&#123; users &#123; id name email role &#125; &#125;</code>
                </CardDescription>
              </div>
            </CardHeader>
            <CardContent>
              {loading ? (
                <div className="flex items-center justify-center py-8 text-muted-foreground">
                  <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                  Mengambil data pengguna dari GraphQL...
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Nama</TableHead>
                      <TableHead>Email</TableHead>
                      <TableHead>Role</TableHead>
                      <TableHead>ID</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {usersList.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                          Belum ada pengguna.
                        </TableCell>
                      </TableRow>
                    ) : (
                      usersList.map(user => (
                        <TableRow key={user.id}>
                          <TableCell className="font-medium">{user.name}</TableCell>
                          <TableCell>{user.email}</TableCell>
                          <TableCell>
                            <Badge variant={user.role === 'admin' ? 'default' : 'secondary'}>
                              {user.role}
                            </Badge>
                          </TableCell>
                          <TableCell className="font-mono text-xs text-muted-foreground">
                            {user.id}
                          </TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </main>
      </div>
    </div>
  );
}
