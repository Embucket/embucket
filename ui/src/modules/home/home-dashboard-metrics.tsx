import { Database, DatabaseZap, FolderTree, Table } from 'lucide-react';

import { Separator } from '@/components/ui/separator';
import type { Dashboard } from '@/orval/models';

interface HomeDashboardMetricsProps {
  dashboardData: Dashboard;
}

export function HomeDashboardMetrics({ dashboardData }: HomeDashboardMetricsProps) {
  return (
    <div className="w-full bg-transparent">
      <div className="flex flex-col md:flex-row">
        <div className="flex-1 rounded-l-md border p-6 text-white [&:not(:last-child)]:border-r-0">
          <div className="flex flex-row items-center justify-between pb-2">
            <h3 className="text-sm font-medium">Total Databases</h3>
            <Database className="text-muted-foreground h-4 w-4" />
          </div>
          <div>
            <div className="text-2xl font-bold">{dashboardData.totalDatabases}</div>
          </div>
        </div>
        <Separator orientation="vertical" className="hidden h-auto bg-zinc-800 md:block" />
        <div className="flex-1 border p-6 text-white [&:not(:last-child)]:border-r-0">
          <div className="flex flex-row items-center justify-between pb-2">
            <h3 className="text-sm font-medium">Total Schemas</h3>
            <FolderTree className="text-muted-foreground h-4 w-4" />
          </div>
          <div>
            <div className="text-2xl font-bold">{dashboardData.totalSchemas}</div>
          </div>
        </div>
        <Separator orientation="vertical" className="hidden h-auto bg-zinc-800 md:block" />
        <div className="flex-1 border p-6 text-white [&:not(:last-child)]:border-r-0">
          <div className="flex flex-row items-center justify-between pb-2">
            <h3 className="text-sm font-medium">Total Tables</h3>
            <Table className="text-muted-foreground h-4 w-4" />
          </div>
          <div>
            <div className="text-2xl font-bold">{dashboardData.totalTables}</div>
          </div>
        </div>
        <Separator orientation="vertical" className="hidden h-auto bg-zinc-800 md:block" />
        <div className="flex-1 rounded-r-md border p-6 text-white [&:not(:last-child)]:border-r-0">
          <div className="flex flex-row items-center justify-between pb-2">
            <h3 className="text-sm font-medium">Total Queries</h3>
            <DatabaseZap className="text-muted-foreground h-4 w-4" />
          </div>
          <div>
            <div className="text-2xl font-bold">{dashboardData.totalQueries}</div>
          </div>
        </div>
      </div>
    </div>
  );
}
