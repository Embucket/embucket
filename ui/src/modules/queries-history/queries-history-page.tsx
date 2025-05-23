import { DatabaseZap } from 'lucide-react';

import { EmptyContainer } from '@/components/empty-container';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { useGetQueries } from '@/orval/queries';

import { PageHeader } from '../shared/page/page-header';
import { QueriesHistoryTable } from './queries-history-table';

export function QueriesHistoryPage() {
  const { data: { items: queries } = {}, isFetching } = useGetQueries();

  return (
    <>
      <PageHeader title="Queries History" />
      <ScrollArea className="h-[calc(100vh-65px-32px)] p-4">
        <div className="flex size-full flex-col">
          {queries?.length ? (
            <ScrollArea tableViewport>
              <QueriesHistoryTable queries={queries} isLoading={isFetching} />
              <ScrollBar orientation="horizontal" />
            </ScrollArea>
          ) : (
            <EmptyContainer
              // TODO: Hardcode
              className="min-h-[calc(100vh-32px-65px-32px)]"
              Icon={DatabaseZap}
              title="No Queries Found"
              description="No queries have been executed yet. Start querying data to see your history here."
            />
          )}
        </div>
        <ScrollBar orientation="vertical" />
      </ScrollArea>
    </>
  );
}
