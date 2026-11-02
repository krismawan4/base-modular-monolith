'use client';

import { useState } from 'react';
import {
  Users, ShieldCheck, Key, Activity,
  LayoutDashboard, Settings, LogOut, Menu, X,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';

// ---------------------------------------------------------------------------
// Dummy data
// ---------------------------------------------------------------------------

const STATS = [
  { label: 'Total User',       value: '128',  icon: Users,       delta: '+12 minggu ini' },
  { label: 'Role Aktif',       value: '4',    icon: ShieldCheck, delta: '2 baru ditambah' },
  { label: 'Permission',       value: '18',   icon: Key,         delta: '3 diassign hari ini' },
  { label: 'Req / menit',      value: '2.4k', icon: Activity,    delta: 'Avg latency 3ms' },
];

const USERS = [
  { id: 'usr-001', email: 'alice@example.com',   role: 'admin',  permissions: ['user:read', 'user:write'], status: 'active' },
  { id: 'usr-002', email: 'bob@example.com',     role: 'member', permissions: ['user:read'],               status: 'active' },
  { id: 'usr-003', email: 'carol@example.com',   role: 'member', permissions: ['user:read'],               status: 'inactive' },
  { id: 'usr-004', email: 'dave@example.com',    role: 'admin',  permissions: ['user:read', 'user:write'], status: 'active' },
  { id: 'usr-005', email: 'eve@example.com',     role: 'member', permissions: ['user:read', 'order:read'], status: 'active' },
];

const NAV = [
  { label: 'Dashboard',  icon: LayoutDashboard, href: '/admin' },
  { label: 'Users',      icon: Users,            href: '/admin/users' },
  { label: 'Roles',      icon: ShieldCheck,      href: '/admin/roles' },
  { label: 'Permissions',icon: Key,              href: '/admin/permissions' },
  { label: 'Settings',   icon: Settings,         href: '/admin/settings' },
];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function AdminDashboard() {
  const [sidebarOpen, setSidebarOpen] = useState(false);

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
          <Badge variant="secondary" className="ml-auto text-[10px]">Admin</Badge>
        </div>

        {/* Nav */}
        <nav className="flex-1 space-y-1 p-4">
          {NAV.map(({ label, icon: Icon, href }) => (
            <a
              key={href}
              href={href}
              className={`
                flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors
                ${href === '/admin'
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
          <button className="flex w-full items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground hover:bg-muted hover:text-foreground transition-colors">
            <LogOut className="h-4 w-4" />
            Logout
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
            <h1 className="text-base font-semibold">Dashboard</h1>
            <p className="text-xs text-muted-foreground">Manajemen User & RBAC</p>
          </div>
          <div className="ml-auto flex items-center gap-3">
            <Badge variant="success">API Online</Badge>
            <div className="h-8 w-8 rounded-full bg-primary text-primary-foreground flex items-center justify-center text-xs font-bold">
              A
            </div>
          </div>
        </header>

        {/* Content */}
        <main className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Stats Cards */}
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {STATS.map(({ label, value, icon: Icon, delta }) => (
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

          {/* User Table */}
          <Card>
            <CardHeader className="flex flex-row items-center justify-between">
              <div>
                <CardTitle>Daftar Pengguna</CardTitle>
                <CardDescription>Seluruh user terdaftar beserta role & permission.</CardDescription>
              </div>
              <Button size="sm">+ Tambah User</Button>
            </CardHeader>
            <CardContent>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Email</TableHead>
                    <TableHead>Role</TableHead>
                    <TableHead>Permissions</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-right">Aksi</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {USERS.map(user => (
                    <TableRow key={user.id}>
                      <TableCell className="font-medium">{user.email}</TableCell>
                      <TableCell>
                        <Badge variant={user.role === 'admin' ? 'default' : 'secondary'}>
                          {user.role}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        <div className="flex flex-wrap gap-1">
                          {user.permissions.map(p => (
                            <Badge key={p} variant="outline" className="text-[10px]">{p}</Badge>
                          ))}
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge variant={user.status === 'active' ? 'success' : 'secondary'}>
                          {user.status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button variant="ghost" size="sm">Edit</Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>
        </main>
      </div>
    </div>
  );
}

