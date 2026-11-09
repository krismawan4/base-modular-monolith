'use client';

import { useState, useEffect, useMemo } from 'react';
import {
  Users, ShieldCheck, Key, Activity, Server, Database,
  LayoutDashboard, Settings, LogOut, Menu, X, Loader2,
  Plus, Pencil, Trash2, Search, RefreshCw, AlertCircle, CheckCircle2,
  ExternalLink, ArrowRight, UserPlus,
  ArrowUpDown, ArrowUp, ArrowDown, Filter, RotateCcw, Calendar,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { useDataTable, type PageSizeOption } from '@/lib/useDataTable';
import { DataTablePagination } from '@/components/ui/DataTablePagination';
import {
  getAuthToken, getAuthUser, clearAuthSession, graphqlRequest,
  type GqlUser, type GqlUsersResponse, type GqlMeResponse,
  type GqlCreateUserResponse, type GqlUpdateUserResponse, type GqlDeleteUserResponse,
  type GqlSystemStatus, type GqlSystemStatusResponse,
  USERS_QUERY, ME_QUERY, CREATE_USER_MUTATION, UPDATE_USER_MUTATION, DELETE_USER_MUTATION,
  SYSTEM_STATUS_QUERY,
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
  const [systemStatus, setSystemStatus] = useState<GqlSystemStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);

  // Column Filters toggle
  const [showColumnFilters, setShowColumnFilters] = useState(false);

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
      const [meData, usersData, statusData] = await Promise.all([
        graphqlRequest<GqlMeResponse>(ME_QUERY, {}, token).catch(() => null),
        graphqlRequest<GqlUsersResponse>(USERS_QUERY, {}, token).catch(() => ({ users: [] })),
        graphqlRequest<GqlSystemStatusResponse>(SYSTEM_STATUS_QUERY, {}, token).catch(() => null),
      ]);

      if (meData?.me) {
        setCurrentUser(meData.me);
      }
      if (usersData?.users) {
        setUsersList(usersData.users);
      }
      if (statusData?.systemStatus) {
        setSystemStatus(statusData.systemStatus);
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

  // Standardized Data Table Hook (Top 1000, Sorting tiap kolom, Search global, Search spesifik kolom, Paginasi)
  const {
    paginatedData: paginatedUsers,
    totalRaw,
    totalFiltered,
    currentPage,
    pageSize,
    totalPages,
    pageSizeOptions,
    startItem,
    endItem,
    pageNumbers,
    setPage,
    setPageSize,
    nextPage,
    prevPage,
    canNextPage,
    canPrevPage,
    globalSearch,
    setGlobalSearch,
    columnFilters,
    setColumnFilter,
    clearColumnFilters,
    hasActiveFilters,
    sortConfig,
    toggleSort,
    resetAll: resetTable,
  } = useDataTable<GqlUser>(usersList, {
    initialPageSize: 10,
    searchableColumns: ['name', 'email', 'role', 'id'],
    defaultSort: { key: 'createdAt', direction: 'desc' },
  });

  // Top recent 5 users for Dashboard
  const recentUsers = useMemo(() => {
    return usersList.slice(0, 5);
  }, [usersList]);

  const stats = [
    { label: 'Total User (Top 1000)',     value: String(usersList.length), icon: Users, delta: 'Maks. 1000 data terbaru' },
    { label: 'User Role',                 value: currentUser?.role || 'Admin', icon: ShieldCheck, delta: 'Role terautentikasi' },
    { label: 'Mode Arsitektur',           value: systemStatus?.architecture || 'Modular Monolith', icon: Server, delta: systemStatus?.usersMode || 'In-Process (Port 8000)' },
    { label: 'Status Sesi',               value: 'Aktif',    icon: Activity, delta: 'Bearer JWT Token' },
  ];

  const renderSortIcon = (key: keyof GqlUser) => {
    if (sortConfig.key !== key || !sortConfig.direction) {
      return <ArrowUpDown className="h-3.5 w-3.5 text-muted-foreground/50 transition-colors" />;
    }
    if (sortConfig.direction === 'asc') {
      return <ArrowUp className="h-3.5 w-3.5 text-primary font-bold" />;
    }
    return <ArrowDown className="h-3.5 w-3.5 text-primary font-bold" />;
  };

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
            <p className="text-xs text-muted-foreground flex items-center gap-1.5 flex-wrap">
              <span>Mode: <strong className="font-medium text-foreground">{systemStatus?.architecture || 'Modular Monolith'}</strong></span>
              <span>•</span>
              <span>Users: <code className="text-[11px] bg-muted px-1.5 py-0.5 rounded text-foreground">{systemStatus?.usersMode || 'In-Process (Modular Monolith)'}</code></span>
              <span>•</span>
              <span>Endpoint: <code className="text-foreground">{systemStatus?.graphqlEndpoint || '/graphql'}</code></span>
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
                      {systemStatus?.message || 'Arsitektur Modular Monolith (In-Process) siap scale ke gRPC'}
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
                        <div className="text-sm font-medium">Komponen Modul Users</div>
                        <div className="text-xs text-muted-foreground">
                          {systemStatus?.usersMode || 'In-Process (Modular Monolith)'}
                        </div>
                      </div>
                      <Badge variant={systemStatus?.usersMode?.includes('gRPC') ? 'default' : 'secondary'}>
                        {systemStatus?.usersMode?.includes('gRPC') ? 'gRPC Microservice' : 'In-Process (Monolith)'}
                      </Badge>
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
                  <div className="flex items-center gap-2">
                    <CardTitle>Manajemen Pengguna (CRUD Users)</CardTitle>
                    <Badge variant="secondary" className="text-[11px] font-mono">
                      Top 1000 Terbaru
                    </Badge>
                  </div>
                  <CardDescription>
                    Standarisasi getData: Menampilkan Top 1000 data terbaru (ORDER BY created_at DESC) dengan pencarian global & spesifik kolom, sorting tiap kolom, dan paginasi client-side.
                  </CardDescription>
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={fetchUsers}
                    disabled={refreshing}
                    className="gap-1.5"
                    title="Muat ulang data terbaru dari backend"
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
                {/* ── Toolbar: Global Search + Filter Actions ── */}
                <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
                  <div className="flex items-center gap-2 flex-1 max-w-md">
                    {/* Global Search Bar */}
                    <div className="relative w-full">
                      <Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" />
                      <input
                        type="text"
                        placeholder="Cari global (nama, email, role, ID)..."
                        value={globalSearch}
                        onChange={e => setGlobalSearch(e.target.value)}
                        className="flex h-9 w-full rounded-md border border-input bg-background pl-9 pr-8 py-1 text-sm shadow-sm transition-colors placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                      />
                      {globalSearch && (
                        <button
                          onClick={() => setGlobalSearch('')}
                          className="absolute right-2.5 top-2.5 text-muted-foreground hover:text-foreground"
                          title="Hapus pencarian global"
                        >
                          <X className="h-4 w-4" />
                        </button>
                      )}
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {/* Toggle Specific Column Search */}
                    <Button
                      variant={showColumnFilters ? 'secondary' : 'outline'}
                      size="sm"
                      onClick={() => setShowColumnFilters(v => !v)}
                      className="gap-1.5 text-xs h-9"
                    >
                      <Filter className="h-3.5 w-3.5" />
                      <span>Filter Spesifik Kolom</span>
                      {Object.keys(columnFilters).length > 0 && (
                        <span className="ml-1 rounded-full bg-primary text-primary-foreground px-1.5 py-0.2 text-[10px] font-bold">
                          {Object.keys(columnFilters).length}
                        </span>
                      )}
                    </Button>

                    {/* Reset All Filters */}
                    {hasActiveFilters && (
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={resetTable}
                        className="gap-1.5 text-xs h-9 text-muted-foreground hover:text-foreground"
                        title="Reset semua filter dan pencarian"
                      >
                        <RotateCcw className="h-3.5 w-3.5" />
                        <span className="hidden sm:inline">Reset Filter</span>
                      </Button>
                    )}
                  </div>
                </div>

                {/* ── Box Filter Spesifik Per Kolom (Collapsible) ── */}
                {showColumnFilters && (
                  <div className="p-3.5 rounded-lg border bg-muted/20 space-y-2 animate-in fade-in-50 duration-200">
                    <div className="flex items-center justify-between text-xs font-semibold text-muted-foreground mb-1">
                      <span>Pencarian Spesifik Kolom (Column-Specific Filters)</span>
                      {Object.keys(columnFilters).length > 0 && (
                        <button
                          onClick={clearColumnFilters}
                          className="text-[11px] text-primary hover:underline"
                        >
                          Bersihkan Kolom
                        </button>
                      )}
                    </div>
                    <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-4 gap-2.5">
                      {/* Search Nama */}
                      <div>
                        <label className="text-[11px] font-medium text-muted-foreground mb-1 block">
                          Cari Nama
                        </label>
                        <input
                          type="text"
                          placeholder="Filter nama..."
                          value={columnFilters.name || ''}
                          onChange={e => setColumnFilter('name', e.target.value)}
                          className="h-8 w-full rounded-md border border-input bg-background px-2.5 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                        />
                      </div>

                      {/* Search Email */}
                      <div>
                        <label className="text-[11px] font-medium text-muted-foreground mb-1 block">
                          Cari Email
                        </label>
                        <input
                          type="text"
                          placeholder="Filter email..."
                          value={columnFilters.email || ''}
                          onChange={e => setColumnFilter('email', e.target.value)}
                          className="h-8 w-full rounded-md border border-input bg-background px-2.5 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                        />
                      </div>

                      {/* Filter Role */}
                      <div>
                        <label className="text-[11px] font-medium text-muted-foreground mb-1 block">
                          Filter Role
                        </label>
                        <select
                          value={columnFilters.role || ''}
                          onChange={e => setColumnFilter('role', e.target.value)}
                          className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer"
                        >
                          <option value="">Semua Role</option>
                          <option value="admin">Admin</option>
                          <option value="member">Member</option>
                        </select>
                      </div>

                      {/* Search ID */}
                      <div>
                        <label className="text-[11px] font-medium text-muted-foreground mb-1 block">
                          Cari User ID
                        </label>
                        <input
                          type="text"
                          placeholder="Filter UUID..."
                          value={columnFilters.id || ''}
                          onChange={e => setColumnFilter('id', e.target.value)}
                          className="h-8 w-full rounded-md border border-input bg-background px-2.5 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring font-mono"
                        />
                      </div>
                    </div>
                  </div>
                )}

                {/* ── Table & Loading States ── */}
                {loading ? (
                  <div className="flex items-center justify-center py-12 text-muted-foreground">
                    <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                    Mengambil data pengguna dari GraphQL...
                  </div>
                ) : (
                  <div className="rounded-md border overflow-hidden">
                    <Table>
                      <TableHeader>
                        <TableRow className="bg-muted/40 hover:bg-muted/40">
                          {/* Kolom Nama */}
                          <TableHead>
                            <button
                              onClick={() => toggleSort('name')}
                              className="group flex items-center gap-1.5 font-semibold text-foreground hover:text-primary transition-colors cursor-pointer select-none"
                              title="Urutkan berdasarkan Nama"
                            >
                              <span>Nama</span>
                              {renderSortIcon('name')}
                            </button>
                          </TableHead>

                          {/* Kolom Email */}
                          <TableHead>
                            <button
                              onClick={() => toggleSort('email')}
                              className="group flex items-center gap-1.5 font-semibold text-foreground hover:text-primary transition-colors cursor-pointer select-none"
                              title="Urutkan berdasarkan Email"
                            >
                              <span>Email</span>
                              {renderSortIcon('email')}
                            </button>
                          </TableHead>

                          {/* Kolom Role */}
                          <TableHead>
                            <button
                              onClick={() => toggleSort('role')}
                              className="group flex items-center gap-1.5 font-semibold text-foreground hover:text-primary transition-colors cursor-pointer select-none"
                              title="Urutkan berdasarkan Role"
                            >
                              <span>Role</span>
                              {renderSortIcon('role')}
                            </button>
                          </TableHead>

                          {/* Kolom Waktu Dibuat */}
                          <TableHead className="hidden lg:table-cell">
                            <button
                              onClick={() => toggleSort('createdAt')}
                              className="group flex items-center gap-1.5 font-semibold text-foreground hover:text-primary transition-colors cursor-pointer select-none"
                              title="Urutkan berdasarkan Waktu Dibuat"
                            >
                              <span>Waktu Dibuat</span>
                              {renderSortIcon('createdAt')}
                            </button>
                          </TableHead>

                          {/* Kolom ID */}
                          <TableHead className="hidden md:table-cell">
                            <button
                              onClick={() => toggleSort('id')}
                              className="group flex items-center gap-1.5 font-semibold text-foreground hover:text-primary transition-colors cursor-pointer select-none"
                              title="Urutkan berdasarkan ID"
                            >
                              <span>ID</span>
                              {renderSortIcon('id')}
                            </button>
                          </TableHead>

                          <TableHead className="text-right font-semibold text-foreground">Aksi</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {paginatedUsers.length === 0 ? (
                          <TableRow>
                            <TableCell colSpan={6} className="text-center py-10 text-muted-foreground">
                              <div className="flex flex-col items-center justify-center gap-2">
                                <AlertCircle className="h-6 w-6 text-muted-foreground/60" />
                                <span className="text-sm">
                                  {hasActiveFilters
                                    ? 'Tidak ada data pengguna yang sesuai dengan kriteria pencarian atau filter.'
                                    : 'Belum ada pengguna terdaftar di sistem.'}
                                </span>
                                {hasActiveFilters && (
                                  <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={resetTable}
                                    className="mt-2 text-xs"
                                  >
                                    Reset Semua Filter
                                  </Button>
                                )}
                              </div>
                            </TableCell>
                          </TableRow>
                        ) : (
                          paginatedUsers.map(user => {
                            const isSelf = currentUser?.id === user.id;
                            const formattedDate = user.createdAt
                              ? new Date(user.createdAt).toLocaleString('id-ID', {
                                  dateStyle: 'medium',
                                  timeStyle: 'short',
                                })
                              : '-';

                            return (
                              <TableRow key={user.id}>
                                <TableCell className="font-medium">
                                  <div className="flex items-center gap-2">
                                    <span>{user.name}</span>
                                    {isSelf && (
                                      <Badge variant="outline" className="text-[10px] py-0 border-primary text-primary">
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
                                <TableCell className="text-xs text-muted-foreground hidden lg:table-cell">
                                  {formattedDate}
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

                    {/* ── Standarisasi Paginasi Client-Side (10, 15, 30, 50, 100) ── */}
                    {totalFiltered > 0 && (
                      <DataTablePagination
                        currentPage={currentPage}
                        totalPages={totalPages}
                        pageSize={pageSize}
                        pageSizeOptions={pageSizeOptions}
                        totalFiltered={totalFiltered}
                        totalRaw={totalRaw}
                        startItem={startItem}
                        endItem={endItem}
                        pageNumbers={pageNumbers}
                        onPageChange={setPage}
                        onPageSizeChange={setPageSize}
                        onNextPage={nextPage}
                        onPrevPage={prevPage}
                        canNextPage={canNextPage}
                        canPrevPage={canPrevPage}
                      />
                    )}
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
