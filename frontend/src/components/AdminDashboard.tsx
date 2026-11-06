'use client';

import { useState, useEffect, useMemo } from 'react';
import {
  Users, ShieldCheck, Key, Activity, Server, Database,
  LayoutDashboard, Settings, LogOut, Menu, X, Loader2,
  Plus, Pencil, Trash2, Search, RefreshCw, AlertCircle, CheckCircle2,
  ExternalLink, ArrowRight, UserPlus,
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
  type GqlCreateUserResponse, type GqlUpdateUserResponse, type GqlDeleteUserResponse,
  USERS_QUERY, ME_QUERY, CREATE_USER_MUTATION, UPDATE_USER_MUTATION, DELETE_USER_MUTATION,
} from '@/lib/graphql';

interface AdminDashboardProps {
  initialTab?: 'dashboard' | 'users' | 'settings';
}

const NAV_ITEMS = [
  { id: 'dashboard', label: 'Dashboard', icon: LayoutDashboard, href: '/admin/dashboard/' },
  { id: 'users',     label: 'Users',     icon: Users,           href: '/admin/users/' },
  { id: 'settings',  label: 'Settings',  icon: Settings,        href: '#settings' },
] as const;

export default function AdminDashboard({ initialTab = 'dashboard' }: AdminDashboardProps) {
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [currentTab, setCurrentTab] = useState<'dashboard' | 'users' | 'settings'>(initialTab);
  const [currentUser, setCurrentUser] = useState<GqlUser | null>(null);
  const [usersList, setUsersList] = useState<GqlUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  // Search & Filter (Users Tab)
  const [searchQuery, setSearchQuery] = useState('');

  // Notification state
  const [notification, setNotification] = useState<{
    type: 'success' | 'error';
    message: string;
  } | null>(null);

  // Modal States (User CRUD)
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<GqlUser | null>(null);
  const [formName, setFormName] = useState('');
  const [formEmail, setFormEmail] = useState('');
  const [formPassword, setFormPassword] = useState('');
  const [formRole, setFormRole] = useState('member');
  const [formSubmitting, setFormSubmitting] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  // Delete Confirmation Modal
  const [deleteModalOpen, setDeleteModalOpen] = useState(false);
  const [userToDelete, setUserToDelete] = useState<GqlUser | null>(null);
  const [deleteSubmitting, setDeleteSubmitting] = useState(false);

  const fetchUsers = async () => {
    const token = getAuthToken();
    if (!token) return;

    try {
      setRefreshing(true);
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
      setRefreshing(false);
      setLoading(false);
    }
  };

  useEffect(() => {
    const token = getAuthToken();
    if (!token) {
      window.location.href = '/login/';
      return;
    }

    const cached = getAuthUser();
    if (cached) setCurrentUser(cached);

    fetchUsers();
  }, []);

  // Sync tab if URL changes or initialTab provided
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const path = window.location.pathname;
      if (path.includes('/admin/users')) {
        setCurrentTab('users');
      } else if (path.includes('/admin/dashboard')) {
        setCurrentTab('dashboard');
      }
    }
  }, []);

  // Auto-dismiss notification
  useEffect(() => {
    if (notification) {
      const timer = setTimeout(() => {
        setNotification(null);
      }, 4000);
      return () => clearTimeout(timer);
    }
  }, [notification]);

  const handleLogout = () => {
    clearAuthSession();
    window.location.href = '/login/';
  };

  // Open Create Modal
  const handleOpenCreateModal = () => {
    setEditingUser(null);
    setFormName('');
    setFormEmail('');
    setFormPassword('');
    setFormRole('member');
    setFormError(null);
    setIsModalOpen(true);
  };

  // Open Edit Modal
  const handleOpenEditModal = (user: GqlUser) => {
    setEditingUser(user);
    setFormName(user.name);
    setFormEmail(user.email);
    setFormPassword('');
    setFormRole(user.role || 'member');
    setFormError(null);
    setIsModalOpen(true);
  };

  // Handle Form Submit (Create or Update)
  const handleFormSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);
    setFormSubmitting(true);

    try {
      if (editingUser) {
        // UPDATE USER
        const input: Record<string, any> = {
          name: formName,
          email: formEmail,
          role: formRole,
        };
        if (formPassword.trim()) {
          input.password = formPassword;
        }

        const res = await graphqlRequest<GqlUpdateUserResponse>(UPDATE_USER_MUTATION, {
          id: editingUser.id,
          input,
        });

        if (res?.updateUser) {
          setNotification({
            type: 'success',
            message: `Pengguna "${res.updateUser.name}" berhasil diperbarui.`,
          });
          setIsModalOpen(false);
          await fetchUsers();
        }
      } else {
        // CREATE USER
        const input: Record<string, any> = {
          name: formName,
          email: formEmail,
          role: formRole,
        };
        if (formPassword.trim()) {
          input.password = formPassword;
        }

        const res = await graphqlRequest<GqlCreateUserResponse>(CREATE_USER_MUTATION, {
          input,
        });

        if (res?.createUser) {
          setNotification({
            type: 'success',
            message: `Pengguna baru "${res.createUser.name}" berhasil ditambahkan.`,
          });
          setIsModalOpen(false);
          await fetchUsers();
        }
      }
    } catch (err: any) {
      setFormError(err?.message || 'Gagal menyimpan data pengguna.');
    } finally {
      setFormSubmitting(false);
    }
  };

  // Open Delete Modal
  const handleOpenDeleteModal = (user: GqlUser) => {
    setUserToDelete(user);
    setDeleteModalOpen(true);
  };

  // Handle Delete Confirm
  const handleDeleteConfirm = async () => {
    if (!userToDelete) return;
    setDeleteSubmitting(true);

    try {
      await graphqlRequest<GqlDeleteUserResponse>(DELETE_USER_MUTATION, {
        id: userToDelete.id,
      });

      setNotification({
        type: 'success',
        message: `Pengguna "${userToDelete.name}" berhasil dihapus.`,
      });
      setDeleteModalOpen(false);
      setUserToDelete(null);
      await fetchUsers();
    } catch (err: any) {
      setNotification({
        type: 'error',
        message: err?.message || 'Gagal menghapus pengguna.',
      });
    } finally {
      setDeleteSubmitting(false);
    }
  };

  // Filtered users for Users Table
  const filteredUsers = useMemo(() => {
    if (!searchQuery.trim()) return usersList;
    const query = searchQuery.toLowerCase();
    return usersList.filter(
      u =>
        u.name.toLowerCase().includes(query) ||
        u.email.toLowerCase().includes(query) ||
        u.role.toLowerCase().includes(query) ||
        u.id.toLowerCase().includes(query)
    );
  }, [usersList, searchQuery]);

  // Top recent 5 users for Dashboard
  const recentUsers = useMemo(() => {
    return usersList.slice(0, 5);
  }, [usersList]);

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

        {/* Navigation Items (Single Active Selection) */}
        <nav className="flex-1 space-y-1 p-4">
          {NAV_ITEMS.map(({ id, label, icon: Icon, href }) => {
            const isActive = currentTab === id;
            return (
              <a
                key={id}
                href={href}
                onClick={e => {
                  if (id === 'settings') {
                    e.preventDefault();
                    setCurrentTab('settings');
                  }
                }}
                className={`
                  flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors
                  ${isActive
                    ? 'bg-primary text-primary-foreground font-semibold shadow-sm'
                    : 'text-muted-foreground hover:bg-muted hover:text-foreground'}
                `}
              >
                <Icon className="h-4 w-4" />
                {label}
              </a>
            );
          })}
        </nav>

        {/* Quick External Link */}
        <div className="px-4 py-2 border-t">
          <a
            href="http://127.0.0.1:8000/graphql"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center justify-between text-xs text-muted-foreground hover:text-foreground py-2 px-2 rounded hover:bg-muted transition-colors"
          >
            <span>GraphQL Playground</span>
            <ExternalLink className="h-3 w-3" />
          </a>
        </div>

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

      {/* ── Main Content Container ── */}
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
            <div className="flex items-center gap-2">
              <h1 className="text-base font-semibold capitalize">
                {currentTab === 'dashboard' && 'Dashboard Overview'}
                {currentTab === 'users' && 'Manajemen Pengguna (CRUD)'}
                {currentTab === 'settings' && 'Pengaturan Sistem'}
              </h1>
              <Badge variant="outline" className="text-[11px] font-mono">
                {currentTab === 'dashboard' && '/admin/dashboard'}
                {currentTab === 'users' && '/admin/users'}
                {currentTab === 'settings' && '/admin/settings'}
              </Badge>
            </div>
            <p className="text-xs text-muted-foreground">
              Protokol Aktif: GraphQL <code>/graphql</code> & gRPC <code>:8001</code>
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

        {/* Main Body */}
        <main className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Notification Toast/Banner */}
          {notification && (
            <div
              className={`flex items-center justify-between gap-2 rounded-lg p-4 text-sm border shadow-sm transition-all ${
                notification.type === 'success'
                  ? 'bg-emerald-50 text-emerald-800 border-emerald-200'
                  : 'bg-destructive/10 text-destructive border-destructive/20'
              }`}
            >
              <div className="flex items-center gap-2">
                {notification.type === 'success' ? (
                  <CheckCircle2 className="h-5 w-5 shrink-0 text-emerald-600" />
                ) : (
                  <AlertCircle className="h-5 w-5 shrink-0 text-destructive" />
                )}
                <span className="font-medium">{notification.message}</span>
              </div>
              <button
                onClick={() => setNotification(null)}
                className="text-muted-foreground hover:text-foreground text-xs"
              >
                Tutup
              </button>
            </div>
          )}

          {/* ═══════════════════════════════════════════════════════════════ */}
          {/* TAB 1: DASHBOARD OVERVIEW                                        */}
          {/* ═══════════════════════════════════════════════════════════════ */}
          {currentTab === 'dashboard' && (
            <div className="space-y-6">
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

              {/* System Architecture & Status Overview */}
              <div className="grid gap-6 md:grid-cols-2">
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                      <Server className="h-4 w-4 text-primary" />
                      Status Komponen Backend & Layanan
                    </CardTitle>
                    <CardDescription>
                      Arsitektur Modular Monolith dengan gRPC Microservices
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="flex items-center justify-between p-3 rounded-lg border bg-muted/20">
                      <div>
                        <div className="text-sm font-medium">GraphQL Gateway Server</div>
                        <div className="text-xs text-muted-foreground">Port 8000 (Async-GraphQL + Axum)</div>
                      </div>
                      <Badge variant="default">Online</Badge>
                    </div>

                    <div className="flex items-center justify-between p-3 rounded-lg border bg-muted/20">
                      <div>
                        <div className="text-sm font-medium">Users Microservice (gRPC)</div>
                        <div className="text-xs text-muted-foreground">Port 8001 (Tonic Protobuf Service)</div>
                      </div>
                      <Badge variant="default">Terhubung</Badge>
                    </div>

                    <div className="flex items-center justify-between p-3 rounded-lg border bg-muted/20">
                      <div>
                        <div className="text-sm font-medium">PostgreSQL & In-Memory Store</div>
                        <div className="text-xs text-muted-foreground">Persistensi Data Pengguna & RBAC</div>
                      </div>
                      <Badge variant="secondary">Aktif</Badge>
                    </div>

                    <div className="flex items-center justify-between p-3 rounded-lg border bg-muted/20">
                      <div>
                        <div className="text-sm font-medium">Redis Token Blacklist</div>
                        <div className="text-xs text-muted-foreground">Revokasi JWT Logout Sesi</div>
                      </div>
                      <Badge variant="secondary">Siap</Badge>
                    </div>
                  </CardContent>
                </Card>

                {/* Quick Actions Card */}
                <Card>
                  <CardHeader>
                    <CardTitle className="text-base flex items-center gap-2">
                      <Activity className="h-4 w-4 text-primary" />
                      Aksi Cepat Admin
                    </CardTitle>
                    <CardDescription>Pintasan untuk operasi sistem dan manajemen data</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <a
                      href="/admin/users/"
                      className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors group"
                    >
                      <div className="flex items-center gap-3">
                        <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center">
                          <Users className="h-4 w-4" />
                        </div>
                        <div>
                          <div className="text-sm font-medium group-hover:text-primary transition-colors">
                            Buka Manajemen Pengguna (CRUD)
                          </div>
                          <div className="text-xs text-muted-foreground">
                            Tambah, cari, ubah, dan hapus data user via GraphQL
                          </div>
                        </div>
                      </div>
                      <ArrowRight className="h-4 w-4 text-muted-foreground group-hover:translate-x-1 transition-all" />
                    </a>

                    <button
                      onClick={handleOpenCreateModal}
                      className="w-full text-left flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors group"
                    >
                      <div className="flex items-center gap-3">
                        <div className="h-9 w-9 rounded-md bg-emerald-500/10 text-emerald-600 flex items-center justify-center">
                          <UserPlus className="h-4 w-4" />
                        </div>
                        <div>
                          <div className="text-sm font-medium group-hover:text-emerald-600 transition-colors">
                            Tambah Pengguna Baru Sekarang
                          </div>
                          <div className="text-xs text-muted-foreground">
                            Buat akun admin atau member baru langsung dari dashboard
                          </div>
                        </div>
                      </div>
                      <Plus className="h-4 w-4 text-muted-foreground" />
                    </button>

                    <a
                      href="http://127.0.0.1:8000/graphql"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors group"
                    >
                      <div className="flex items-center gap-3">
                        <div className="h-9 w-9 rounded-md bg-purple-500/10 text-purple-600 flex items-center justify-center">
                          <Key className="h-4 w-4" />
                        </div>
                        <div>
                          <div className="text-sm font-medium group-hover:text-purple-600 transition-colors">
                            Buka GraphiQL Playground
                          </div>
                          <div className="text-xs text-muted-foreground">
                            IDE interaktif untuk query dan test mutasi GraphQL
                          </div>
                        </div>
                      </div>
                      <ExternalLink className="h-4 w-4 text-muted-foreground" />
                    </a>
                  </CardContent>
                </Card>
              </div>

              {/* Recent Users Preview Card */}
              <Card>
                <CardHeader className="flex flex-row items-center justify-between">
                  <div>
                    <CardTitle className="text-base">Pengguna Terbaru (Snapshot)</CardTitle>
                    <CardDescription>
                      Daftar 5 akun pengguna terbaru yang tercatat di backend
                    </CardDescription>
                  </div>
                  <a href="/admin/users/">
                    <Button variant="outline" size="sm" className="gap-1.5 text-xs">
                      <span>Kelola Semua Pengguna</span>
                      <ArrowRight className="h-3.5 w-3.5" />
                    </Button>
                  </a>
                </CardHeader>
                <CardContent>
                  {loading ? (
                    <div className="flex items-center justify-center py-6 text-muted-foreground">
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Memuat data pengguna...
                    </div>
                  ) : (
                    <div className="rounded-md border">
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>Nama</TableHead>
                            <TableHead>Email</TableHead>
                            <TableHead>Role</TableHead>
                            <TableHead className="hidden sm:table-cell">ID</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {recentUsers.length === 0 ? (
                            <TableRow>
                              <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                                Belum ada pengguna.
                              </TableCell>
                            </TableRow>
                          ) : (
                            recentUsers.map(user => (
                              <TableRow key={user.id}>
                                <TableCell className="font-medium">{user.name}</TableCell>
                                <TableCell>{user.email}</TableCell>
                                <TableCell>
                                  <Badge variant={user.role === 'admin' ? 'default' : 'secondary'}>
                                    {user.role}
                                  </Badge>
                                </TableCell>
                                <TableCell className="font-mono text-xs text-muted-foreground hidden sm:table-cell">
                                  {user.id}
                                </TableCell>
                              </TableRow>
                            ))
                          )}
                        </TableBody>
                      </Table>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          )}

          {/* ═══════════════════════════════════════════════════════════════ */}
          {/* TAB 2: USERS CRUD MANAGEMENT                                     */}
          {/* ═══════════════════════════════════════════════════════════════ */}
          {currentTab === 'users' && (
            <Card>
              <CardHeader className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
                <div>
                  <CardTitle>Manajemen Pengguna (CRUD Users)</CardTitle>
                  <CardDescription>
                    Kelola data pengguna secara langsung melalui mutasi & query GraphQL
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={fetchUsers}
                    disabled={refreshing}
                    className="gap-1.5"
                  >
                    <RefreshCw className={`h-4 w-4 ${refreshing ? 'animate-spin' : ''}`} />
                    <span className="hidden sm:inline">Refresh</span>
                  </Button>
                  <Button
                    size="sm"
                    onClick={handleOpenCreateModal}
                    className="gap-1.5"
                  >
                    <Plus className="h-4 w-4" />
                    Tambah Pengguna
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                {/* Search Bar */}
                <div className="flex items-center gap-2 max-w-sm">
                  <div className="relative w-full">
                    <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
                    <input
                      type="text"
                      placeholder="Cari berdasarkan nama, email, role..."
                      value={searchQuery}
                      onChange={e => setSearchQuery(e.target.value)}
                      className="flex h-9 w-full rounded-md border border-input bg-background pl-9 pr-3 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                    />
                  </div>
                </div>

                {/* Table */}
                {loading ? (
                  <div className="flex items-center justify-center py-12 text-muted-foreground">
                    <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                    Mengambil data pengguna dari GraphQL...
                  </div>
                ) : (
                  <div className="rounded-md border">
                    <Table>
                      <TableHeader>
                        <TableRow>
                          <TableHead>Nama</TableHead>
                          <TableHead>Email</TableHead>
                          <TableHead>Role</TableHead>
                          <TableHead className="hidden md:table-cell">ID</TableHead>
                          <TableHead className="text-right">Aksi</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {filteredUsers.length === 0 ? (
                          <TableRow>
                            <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                              {searchQuery ? 'Tidak ada pengguna yang cocok dengan pencarian.' : 'Belum ada pengguna.'}
                            </TableCell>
                          </TableRow>
                        ) : (
                          filteredUsers.map(user => {
                            const isSelf = currentUser?.id === user.id;
                            return (
                              <TableRow key={user.id}>
                                <TableCell className="font-medium">
                                  <div className="flex items-center gap-2">
                                    <span>{user.name}</span>
                                    {isSelf && (
                                      <Badge variant="outline" className="text-[10px] py-0">
                                        Anda
                                      </Badge>
                                    )}
                                  </div>
                                </TableCell>
                                <TableCell>{user.email}</TableCell>
                                <TableCell>
                                  <Badge variant={user.role === 'admin' ? 'default' : 'secondary'}>
                                    {user.role}
                                  </Badge>
                                </TableCell>
                                <TableCell className="font-mono text-xs text-muted-foreground hidden md:table-cell">
                                  {user.id}
                                </TableCell>
                                <TableCell className="text-right">
                                  <div className="flex items-center justify-end gap-1.5">
                                    <Button
                                      variant="outline"
                                      size="sm"
                                      onClick={() => handleOpenEditModal(user)}
                                      className="h-8 px-2 text-xs gap-1"
                                      title="Edit Pengguna"
                                    >
                                      <Pencil className="h-3.5 w-3.5" />
                                      <span className="hidden sm:inline">Edit</span>
                                    </Button>
                                    <Button
                                      variant="destructive"
                                      size="sm"
                                      onClick={() => handleOpenDeleteModal(user)}
                                      className="h-8 px-2 text-xs gap-1"
                                      title={isSelf ? 'Tidak dapat menghapus akun sendiri' : 'Hapus Pengguna'}
                                      disabled={isSelf}
                                    >
                                      <Trash2 className="h-3.5 w-3.5" />
                                      <span className="hidden sm:inline">Hapus</span>
                                    </Button>
                                  </div>
                                </TableCell>
                              </TableRow>
                            );
                          })
                        )}
                      </TableBody>
                    </Table>
                  </div>
                )}
              </CardContent>
            </Card>
          )}

          {/* ═══════════════════════════════════════════════════════════════ */}
          {/* TAB 3: SETTINGS                                                  */}
          {/* ═══════════════════════════════════════════════════════════════ */}
          {currentTab === 'settings' && (
            <Card>
              <CardHeader>
                <CardTitle>Pengaturan & Info Sesi</CardTitle>
                <CardDescription>Informasi detail akun terautentikasi dan konfigurasi API</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="p-4 rounded-lg border bg-muted/20 space-y-1">
                    <div className="text-xs text-muted-foreground font-medium">User Terautentikasi</div>
                    <div className="text-sm font-semibold">{currentUser?.name || '-'}</div>
                    <div className="text-xs text-muted-foreground">{currentUser?.email || '-'}</div>
                  </div>
                  <div className="p-4 rounded-lg border bg-muted/20 space-y-1">
                    <div className="text-xs text-muted-foreground font-medium">Role & Hak Akses</div>
                    <div className="text-sm font-semibold capitalize">{currentUser?.role || '-'}</div>
                    <div className="text-xs text-muted-foreground">Akses Penuh Manajemen Pengguna</div>
                  </div>
                </div>

                <div className="p-4 rounded-lg border bg-muted/20 space-y-2">
                  <div className="text-xs text-muted-foreground font-medium">Endpoint GraphQL</div>
                  <div className="font-mono text-xs bg-background p-2 rounded border">
                    http://127.0.0.1:8000/graphql
                  </div>
                </div>
              </CardContent>
            </Card>
          )}
        </main>
      </div>

      {/* ── Modal Create / Edit User ── */}
      {isModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 animate-in fade-in duration-150">
          <div className="w-full max-w-md rounded-lg border bg-white p-6 shadow-xl space-y-4">
            <div className="flex items-center justify-between border-b pb-3">
              <h2 className="text-lg font-semibold">
                {editingUser ? 'Edit Pengguna' : 'Tambah Pengguna Baru'}
              </h2>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-muted-foreground hover:text-foreground"
              >
                <X className="h-5 w-5" />
              </button>
            </div>

            {formError && (
              <div className="flex items-center gap-2 rounded-md bg-destructive/10 p-3 text-sm text-destructive border border-destructive/20">
                <AlertCircle className="h-4 w-4 shrink-0" />
                <span>{formError}</span>
              </div>
            )}

            <form onSubmit={handleFormSubmit} className="space-y-4">
              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="name">
                  Nama Lengkap
                </label>
                <input
                  id="name"
                  type="text"
                  required
                  placeholder="Contoh: Budi Santoso"
                  value={formName}
                  onChange={e => setFormName(e.target.value)}
                  className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="email">
                  Alamat Email
                </label>
                <input
                  id="email"
                  type="email"
                  required
                  placeholder="budi@example.com"
                  value={formEmail}
                  onChange={e => setFormEmail(e.target.value)}
                  className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="password">
                  {editingUser ? 'Password Baru (Opsional)' : 'Password'}
                </label>
                <input
                  id="password"
                  type="password"
                  placeholder={editingUser ? 'Kosongkan jika tidak ingin mengubah' : 'Minimal 6 karakter'}
                  value={formPassword}
                  onChange={e => setFormPassword(e.target.value)}
                  className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                />
                {editingUser && (
                  <p className="text-[11px] text-muted-foreground">
                    Biarkan kosong jika tetap menggunakan password saat ini.
                  </p>
                )}
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium" htmlFor="role">
                  Role Pengguna
                </label>
                <select
                  id="role"
                  value={formRole}
                  onChange={e => setFormRole(e.target.value)}
                  className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                >
                  <option value="member">member (Standar)</option>
                  <option value="admin">admin (Akses Penuh)</option>
                  <option value="user">user</option>
                </select>
              </div>

              <div className="flex items-center justify-end gap-2 pt-4 border-t">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setIsModalOpen(false)}
                  disabled={formSubmitting}
                >
                  Batal
                </Button>
                <Button type="submit" disabled={formSubmitting} className="gap-2">
                  {formSubmitting && <Loader2 className="h-4 w-4 animate-spin" />}
                  {editingUser ? 'Simpan Perubahan' : 'Tambah Pengguna'}
                </Button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* ── Modal Delete Confirmation ── */}
      {deleteModalOpen && userToDelete && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4 animate-in fade-in duration-150">
          <div className="w-full max-w-sm rounded-lg border bg-white p-6 shadow-xl space-y-4">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 items-center justify-center rounded-full bg-destructive/10 text-destructive">
                <Trash2 className="h-5 w-5" />
              </div>
              <div>
                <h3 className="font-semibold text-base">Hapus Pengguna</h3>
                <p className="text-xs text-muted-foreground">Tindakan ini tidak dapat dibatalkan</p>
              </div>
            </div>

            <p className="text-sm text-foreground/80">
              Apakah Anda yakin ingin menghapus pengguna <strong className="font-medium text-foreground">{userToDelete.name}</strong> ({userToDelete.email})?
            </p>

            <div className="flex items-center justify-end gap-2 pt-2 border-t">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setDeleteModalOpen(false)}
                disabled={deleteSubmitting}
              >
                Batal
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={handleDeleteConfirm}
                disabled={deleteSubmitting}
                className="gap-1.5"
              >
                {deleteSubmitting && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
                Ya, Hapus
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
