import React from 'react';
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { type PageSizeOption } from '@/lib/useDataTable';

interface DataTablePaginationProps {
  currentPage: number;
  totalPages: number;
  pageSize: PageSizeOption;
  pageSizeOptions: PageSizeOption[];
  totalFiltered: number;
  totalRaw: number;
  startItem: number;
  endItem: number;
  pageNumbers: (number | 'ellipsis')[];
  onPageChange: (page: number) => void;
  onPageSizeChange: (size: PageSizeOption) => void;
  onNextPage: () => void;
  onPrevPage: () => void;
  canNextPage: boolean;
  canPrevPage: boolean;
}

export function DataTablePagination({
  currentPage,
  totalPages,
  pageSize,
  pageSizeOptions,
  totalFiltered,
  totalRaw,
  startItem,
  endItem,
  pageNumbers,
  onPageChange,
  onPageSizeChange,
  onNextPage,
  onPrevPage,
  canNextPage,
  canPrevPage,
}: DataTablePaginationProps) {
  return (
    <div className="flex flex-col sm:flex-row items-center justify-between gap-4 py-3 px-2 border-t text-sm">
      {/* Kiri: Info Jumlah Data & Paginasi */}
      <div className="flex flex-wrap items-center gap-2 text-muted-foreground text-xs sm:text-sm">
        <span>
          Menampilkan <strong className="font-semibold text-foreground">{startItem}</strong> -{' '}
          <strong className="font-semibold text-foreground">{endItem}</strong> dari{' '}
          <strong className="font-semibold text-foreground">{totalFiltered}</strong> data
        </span>
        {totalRaw > totalFiltered && (
          <span className="text-muted-foreground text-xs">
            (difilter dari total {totalRaw} data)
          </span>
        )}
        <span className="inline-flex items-center px-2 py-0.5 rounded text-[11px] bg-muted font-mono font-medium text-muted-foreground">
          Top 1000 Terbaru
        </span>
      </div>

      {/* Kanan: Pilihan Page Size (10, 15, 30, 50, 100) & Tombol Navigasi Halaman */}
      <div className="flex flex-wrap items-center gap-4">
        {/* Page size dropdown */}
        <div className="flex items-center gap-2 text-xs">
          <label htmlFor="pageSizeSelect" className="text-muted-foreground whitespace-nowrap">
            Baris per halaman:
          </label>
          <select
            id="pageSizeSelect"
            value={pageSize}
            onChange={e => onPageSizeChange(Number(e.target.value) as PageSizeOption)}
            className="h-8 rounded-md border border-input bg-background px-2.5 py-1 text-xs font-medium shadow-sm transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring cursor-pointer"
          >
            {pageSizeOptions.map(opt => (
              <option key={opt} value={opt}>
                {opt}
              </option>
            ))}
          </select>
        </div>

        {/* Page navigation buttons */}
        <div className="flex items-center gap-1">
          {/* First page */}
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(1)}
            disabled={!canPrevPage}
            className="h-8 w-8 p-0"
            title="Halaman Pertama"
          >
            <ChevronsLeft className="h-4 w-4" />
          </Button>

          {/* Prev page */}
          <Button
            variant="outline"
            size="sm"
            onClick={onPrevPage}
            disabled={!canPrevPage}
            className="h-8 w-8 p-0"
            title="Halaman Sebelumnya"
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>

          {/* Number buttons */}
          <div className="hidden sm:flex items-center gap-1">
            {pageNumbers.map((p, idx) => {
              if (p === 'ellipsis') {
                return (
                  <span key={`ellipsis-${idx}`} className="px-1 text-xs text-muted-foreground select-none">
                    ...
                  </span>
                );
              }
              const isActive = p === currentPage;
              return (
                <Button
                  key={p}
                  variant={isActive ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => onPageChange(p)}
                  className={`h-8 min-w-[32px] px-2 text-xs font-medium ${
                    isActive ? 'shadow-sm' : ''
                  }`}
                >
                  {p}
                </Button>
              );
            })}
          </div>

          {/* Mobile page indicator */}
          <div className="sm:hidden text-xs px-2 text-muted-foreground">
            {currentPage} / {totalPages}
          </div>

          {/* Next page */}
          <Button
            variant="outline"
            size="sm"
            onClick={onNextPage}
            disabled={!canNextPage}
            className="h-8 w-8 p-0"
            title="Halaman Berikutnya"
          >
            <ChevronRight className="h-4 w-4" />
          </Button>

          {/* Last page */}
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(totalPages)}
            disabled={!canNextPage}
            className="h-8 w-8 p-0"
            title="Halaman Terakhir"
          >
            <ChevronsRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
