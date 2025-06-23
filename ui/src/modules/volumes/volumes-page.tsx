import { useState } from 'react';

import { Box } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { useDebounce } from '@/hooks/use-debounce';
import { useGetVolumes } from '@/orval/volumes';

import { CreateVolumeDialog } from '../shared/create-volume-dialog/create-volume-dialog';
import { PageEmptyContainer } from '../shared/page/page-empty-container';
import { PageHeader } from '../shared/page/page-header';
import { PageScrollArea } from '../shared/page/page-scroll-area';
import { VolumesTable } from './volumes-page-table';
import { VolumesPageToolbar } from './volumes-page-toolbar';

export function VolumesPage() {
  const [opened, setOpened] = useState(false);
  const [search, setSearch] = useState('');
  const debouncedSearch = useDebounce(search, 500);

  const {
    data: { items: volumes } = {},
    isFetching: isFetchingVolumes,
    isLoading: isLoadingVolumes,
    refetch: refetchVolumes,
  } = useGetVolumes({
    search: debouncedSearch,
  });

  return (
    <>
      <PageHeader
        title="Volumes"
        Action={
          <Button size="sm" disabled={isFetchingVolumes} onClick={() => setOpened(true)}>
            Add Volume
          </Button>
        }
      />

      <VolumesPageToolbar
        search={search}
        onSetSearch={setSearch}
        volumes={volumes ?? []}
        isFetchingVolumes={isFetchingVolumes}
        onRefetchVolumes={refetchVolumes}
      />
      {!volumes?.length && !isLoadingVolumes ? (
        <PageEmptyContainer
          Icon={Box}
          variant="toolbar"
          title="No Volumes Found"
          description="No volumes have been created yet. Create a volume to get started."
        />
      ) : (
        <PageScrollArea>
          <VolumesTable volumes={volumes ?? []} isLoading={isLoadingVolumes} />
        </PageScrollArea>
      )}

      <CreateVolumeDialog opened={opened} onSetOpened={setOpened} />
    </>
  );
}
