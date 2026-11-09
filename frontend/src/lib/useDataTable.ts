import { useState, useMemo, useCallback } from 'react';

export type SortDirection = 'asc' | 'desc' | null;

export interface SortConfig<T> {
  key: keyof T | null;
  direction: SortDirection;
}

export type PageSizeOption = 10 | 15 | 30 | 50 | 100;
export const PAGE_SIZE_OPTIONS: PageSizeOption[] = [10, 15, 30, 50, 100];

export interface UseDataTableOptions<T> {
  initialPageSize?: PageSizeOption;
  searchableColumns?: (keyof T)[];
  defaultSort?: {
    key: keyof T;
    direction: 'asc' | 'desc';
  };
}

export interface UseDataTableReturn<T> {
  // Data
  paginatedData: T[];
  totalRaw: number;
  totalFiltered: number;
  
  // Pagination
  currentPage: number;
  pageSize: PageSizeOption;
  totalPages: number;
  pageSizeOptions: PageSizeOption[];
  startItem: number;
  endItem: number;
  pageNumbers: (number | 'ellipsis')[];
  setPage: (page: number) => void;
  setPageSize: (size: PageSizeOption) => void;
  nextPage: () => void;
  prevPage: () => void;
  canNextPage: boolean;
  canPrevPage: boolean;

  // Global Search
  globalSearch: string;
  setGlobalSearch: (query: string) => void;

  // Specific Column Filters
  columnFilters: Partial<Record<keyof T, string>>;
  setColumnFilter: (column: keyof T, value: string) => void;
  clearColumnFilters: () => void;
  hasActiveFilters: boolean;

  // Sorting
  sortConfig: SortConfig<T>;
  toggleSort: (column: keyof T) => void;
  setSort: (column: keyof T, direction: SortDirection) => void;

  // Reset
  resetAll: () => void;
}

export function useDataTable<T extends Record<string, any>>(
  data: T[],
  options: UseDataTableOptions<T> = {}
): UseDataTableReturn<T> {
  const {
    initialPageSize = 10,
    searchableColumns,
    defaultSort = null,
  } = options;

  const [globalSearch, setGlobalSearchState] = useState('');
  const [columnFilters, setColumnFiltersState] = useState<Partial<Record<keyof T, string>>>({});
  const [sortConfig, setSortConfig] = useState<SortConfig<T>>({
    key: defaultSort?.key ?? null,
    direction: defaultSort?.direction ?? null,
  });
  const [pageSize, setPageSizeState] = useState<PageSizeOption>(initialPageSize);
  const [currentPage, setCurrentPage] = useState(1);

  // Setters that reset page to 1
  const setGlobalSearch = useCallback((query: string) => {
    setGlobalSearchState(query);
    setCurrentPage(1);
  }, []);

  const setColumnFilter = useCallback((column: keyof T, value: string) => {
    setColumnFiltersState(prev => {
      const next = { ...prev };
      if (!value || value.trim() === '') {
        delete next[column];
      } else {
        next[column] = value;
      }
      return next;
    });
    setCurrentPage(1);
  }, []);

  const clearColumnFilters = useCallback(() => {
    setColumnFiltersState({});
    setCurrentPage(1);
  }, []);

  const setPageSize = useCallback((size: PageSizeOption) => {
    setPageSizeState(size);
    setCurrentPage(1);
  }, []);

  const setSort = useCallback((column: keyof T, direction: SortDirection) => {
    setSortConfig({ key: column, direction });
  }, []);

  const toggleSort = useCallback((column: keyof T) => {
    setSortConfig(prev => {
      if (prev.key !== column) {
        return { key: column, direction: 'asc' };
      }
      if (prev.direction === 'asc') {
        return { key: column, direction: 'desc' };
      }
      // If desc, cycle back to null (default order)
      return { key: null, direction: null };
    });
  }, []);

  const resetAll = useCallback(() => {
    setGlobalSearchState('');
    setColumnFiltersState({});
    setSortConfig({
      key: defaultSort?.key ?? null,
      direction: defaultSort?.direction ?? null,
    });
    setCurrentPage(1);
  }, [defaultSort]);

  const hasActiveFilters = useMemo(() => {
    return Boolean(
      globalSearch.trim() ||
      Object.values(columnFilters).some(v => v && typeof v === 'string' && v.trim() !== '')
    );
  }, [globalSearch, columnFilters]);

  // 1. Filter data (Column search + Global search)
  const filteredData = useMemo(() => {
    let result = data;

    // Apply specific column filters
    const activeFilters = Object.entries(columnFilters).filter(
      ([_, val]) => val !== undefined && val !== null && String(val).trim() !== ''
    ) as [keyof T, string][];

    if (activeFilters.length > 0) {
      result = result.filter(item => {
        return activeFilters.every(([col, filterVal]) => {
          const itemVal = item[col];
          if (itemVal === undefined || itemVal === null) return false;
          return String(itemVal).toLowerCase().includes(filterVal.toLowerCase().trim());
        });
      });
    }

    // Apply global search across columns
    if (globalSearch.trim()) {
      const query = globalSearch.toLowerCase().trim();
      result = result.filter(item => {
        if (searchableColumns && searchableColumns.length > 0) {
          return searchableColumns.some(col => {
            const val = item[col];
            if (val === undefined || val === null) return false;
            return String(val).toLowerCase().includes(query);
          });
        }
        // Fallback: search all object keys
        return Object.values(item).some(val => {
          if (val === undefined || val === null) return false;
          if (typeof val === 'object') return false;
          return String(val).toLowerCase().includes(query);
        });
      });
    }

    return result;
  }, [data, columnFilters, globalSearch, searchableColumns]);

  // 2. Sort filtered data
  const sortedData = useMemo(() => {
    if (!sortConfig.key || !sortConfig.direction) {
      return filteredData;
    }

    const { key, direction } = sortConfig;
    const modifier = direction === 'asc' ? 1 : -1;

    return [...filteredData].sort((a, b) => {
      const valA = a[key];
      const valB = b[key];

      if (valA === valB) return 0;
      if (valA === undefined || valA === null) return 1;
      if (valB === undefined || valB === null) return -1;

      // Handle numbers
      if (typeof valA === 'number' && typeof valB === 'number') {
        return (valA - valB) * modifier;
      }

      // Handle ISO strings or dates
      const dateA = Date.parse(String(valA));
      const dateB = Date.parse(String(valB));
      if (!isNaN(dateA) && !isNaN(dateB) && isNaN(Number(valA)) && isNaN(Number(valB))) {
        return (dateA - dateB) * modifier;
      }

      // Standard string comparison
      return String(valA).localeCompare(String(valB), undefined, { numeric: true, sensitivity: 'base' }) * modifier;
    });
  }, [filteredData, sortConfig]);

  // 3. Paginate
  const totalFiltered = sortedData.length;
  const totalPages = Math.max(1, Math.ceil(totalFiltered / pageSize));

  // Clamp current page
  const safePage = Math.min(Math.max(1, currentPage), totalPages);

  const startItem = totalFiltered === 0 ? 0 : (safePage - 1) * pageSize + 1;
  const endItem = Math.min(safePage * pageSize, totalFiltered);

  const paginatedData = useMemo(() => {
    const start = (safePage - 1) * pageSize;
    return sortedData.slice(start, start + pageSize);
  }, [sortedData, safePage, pageSize]);

  const canPrevPage = safePage > 1;
  const canNextPage = safePage < totalPages;

  const setPage = useCallback((page: number) => {
    setCurrentPage(Math.min(Math.max(1, page), totalPages));
  }, [totalPages]);

  const nextPage = useCallback(() => {
    if (canNextPage) {
      setCurrentPage(prev => Math.min(prev + 1, totalPages));
    }
  }, [canNextPage, totalPages]);

  const prevPage = useCallback(() => {
    if (canPrevPage) {
      setCurrentPage(prev => Math.max(prev - 1, 1));
    }
  }, [canPrevPage]);

  // Smart page numbers calculation with ellipsis
  const pageNumbers = useMemo(() => {
    const delta = 1;
    const range: number[] = [];
    const rangeWithDots: (number | 'ellipsis')[] = [];
    let l: number | undefined;

    for (let i = 1; i <= totalPages; i++) {
      if (i === 1 || i === totalPages || (i >= safePage - delta && i <= safePage + delta)) {
        range.push(i);
      }
    }

    for (const i of range) {
      if (l) {
        if (i - l === 2) {
          rangeWithDots.push(l + 1);
        } else if (i - l !== 1) {
          rangeWithDots.push('ellipsis');
        }
      }
      rangeWithDots.push(i);
      l = i;
    }

    return rangeWithDots;
  }, [totalPages, safePage]);

  return {
    paginatedData,
    totalRaw: data.length,
    totalFiltered,
    currentPage: safePage,
    pageSize,
    totalPages,
    pageSizeOptions: PAGE_SIZE_OPTIONS,
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
    setSort,
    resetAll,
  };
}
