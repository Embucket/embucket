import { useEffect, useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';
import { useParams } from '@tanstack/react-router';
import { AxiosError } from 'axios';
import { toast } from 'sonner';

import { ResizablePanelGroup } from '@/components/ui/resizable';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { Editor } from '@/modules/editor/editor';
import { useEditorPanelsState } from '@/modules/editor/editor-panels-state-provider';
import { getGetDashboardQueryKey } from '@/orval/dashboard';
import type { QueryRecord } from '@/orval/models';
import { getGetNavigationTreesQueryKey } from '@/orval/navigation-trees';
import { getGetQueriesQueryKey, useCreateQuery, useGetQuery } from '@/orval/queries';

import { EditorResizableHandle, EditorResizablePanel } from '../editor-resizable';
import { useEditorSettingsStore } from '../editor-settings-store';
import { useSaveEditorContent } from '../use-save-editor-conent';
import { EditorCenterBottomPanel } from './editor-center-bottom-panel/editor-center-bottom-panel';
import { EditorCenterPanelFooter } from './editor-center-panel-footer';
import { EditorCenterPanelHeader } from './editor-center-panel-header/editor-center-panel-header';
import { EditorCenterPanelToolbar } from './editor-center-panel-toolbar/editor-center-panel-toolbar';

export function EditorCenterPanel() {
  const { worksheetId } = useParams({ from: '/sql-editor/$worksheetId/' });
  const { save } = useSaveEditorContent();

  const [pollingQueryId, setPollingQueryId] = useState<number>();
  const selectedQueryRecord = useEditorSettingsStore((state) =>
    state.getSelectedQueryRecord(+worksheetId),
  );
  const selectedContext = useEditorSettingsStore((state) => state.selectedContext);
  const setSelectedQueryRecord = useEditorSettingsStore((state) => state.setSelectedQueryRecord);

  const {
    groupRef,
    topRef,
    bottomRef,
    setTopPanelExpanded,
    setBottomPanelExpanded,
    isRightPanelExpanded,
    toggleRightPanel,
  } = useEditorPanelsState();

  const queryClient = useQueryClient();

  const { mutate: createQuery, isPending: createQueryPending } = useCreateQuery({
    mutation: {
      onSuccess: (queryRecord) => {
        if (queryRecord.id) {
          setPollingQueryId(queryRecord.id);
          setSelectedQueryRecord(+worksheetId, queryRecord);
          // Optimistic update
          queryClient.setQueryData(
            getGetQueriesQueryKey({ worksheetId: +worksheetId }),
            (oldData: { items: QueryRecord[] } | undefined) => {
              const oldItems = oldData?.items ?? [];
              return {
                ...oldData,
                items: [queryRecord, ...oldItems],
              };
            },
          );
        }
      },
      onError: (error) => {
        if (error instanceof AxiosError) {
          toast.error(error.message, {
            description: error.response?.data.message,
          });
        }
      },
    },
  });

  const id = selectedQueryRecord?.status === 'running' ? selectedQueryRecord.id : pollingQueryId;

  const { data: queryRecord, isFetching: queryRecordLoading } = useGetQuery(id!, {
    query: {
      enabled: !!id,
      refetchInterval: 500,
      refetchOnWindowFocus: false,
    },
  });

  useEffect(() => {
    if (queryRecord?.status !== 'running' && id && !queryRecordLoading) {
      setPollingQueryId(undefined);

      if (queryRecord?.status === 'successful') {
        if (!isRightPanelExpanded) {
          toggleRightPanel();
        }
        setSelectedQueryRecord(+worksheetId, queryRecord);
        console.log('success', worksheetId, queryRecord.id, queryRecordLoading);
      }
      if (queryRecord?.status === 'failed') {
        toast.error(queryRecord.error);
      }
      Promise.all([
        // Use Promise.all since useEffect isn't async
        queryClient.invalidateQueries({
          queryKey: getGetQueriesQueryKey({ worksheetId: +worksheetId }),
        }),
        queryClient.invalidateQueries({
          queryKey: getGetDashboardQueryKey(),
        }),
        queryClient.invalidateQueries({
          queryKey: getGetNavigationTreesQueryKey(),
        }),
      ]);
    } else {
      console.log('running', worksheetId, queryRecord?.id, queryRecordLoading);
    }
  }, [
    isRightPanelExpanded,
    queryClient,
    queryRecord,
    queryRecordLoading,
    setSelectedQueryRecord,
    toggleRightPanel,
    id,
    worksheetId,
  ]);

  const handleRunQuery = (query: string) => {
    save(+worksheetId);
    const optimisticRecord = {
      id: Date.now(),
      status: 'running',
      query: 'TEST',
      worksheetId: +worksheetId,
      context: '',
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    // Update cache optimistically
    queryClient.setQueryData(getGetQueriesQueryKey(), (old?: QueryRecord[]) => {
      if (!old) return [optimisticRecord];
      return [optimisticRecord, ...old];
    });

    createQuery({
      data: {
        query,
        asyncExec: true,
        worksheetId: +worksheetId,
        context: {
          database: selectedContext.database,
          schema: selectedContext.schema,
        },
      },
    });
  };

  const loading = selectedQueryRecord?.status === 'running' || createQueryPending;

  return (
    <div className="flex h-full flex-col">
      <EditorCenterPanelHeader />

      <EditorCenterPanelToolbar onRunQuery={handleRunQuery} isLoading={loading} />
      <ResizablePanelGroup direction="vertical" ref={groupRef}>
        <EditorResizablePanel
          collapsible
          defaultSize={30}
          minSize={25}
          onCollapse={() => setTopPanelExpanded(false)}
          onExpand={() => setTopPanelExpanded(true)}
          order={1}
          ref={topRef}
        >
          <ScrollArea
            tableViewport
            className="bg-background size-full [&>*>*:first-child]:h-full [&>*>*>*:first-child]:h-full"
          >
            <Editor />
            <ScrollBar orientation="horizontal" />
            <ScrollBar orientation="vertical" />
          </ScrollArea>
        </EditorResizablePanel>

        <EditorResizableHandle />

        <EditorResizablePanel
          collapsible
          defaultSize={70}
          minSize={25}
          onCollapse={() => setBottomPanelExpanded(false)}
          onExpand={() => setBottomPanelExpanded(true)}
          order={2}
          ref={bottomRef}
        >
          <EditorCenterBottomPanel queryRecord={selectedQueryRecord} isLoading={loading} />
        </EditorResizablePanel>
      </ResizablePanelGroup>

      <EditorCenterPanelFooter />
    </div>
  );
}
