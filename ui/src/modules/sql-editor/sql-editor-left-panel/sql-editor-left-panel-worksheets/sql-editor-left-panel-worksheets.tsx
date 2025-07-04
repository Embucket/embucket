import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { SidebarMenu } from '@/components/ui/sidebar';
import type { Worksheet } from '@/orval/models';

import { SqlEditorLeftPanelWorksheet } from './sql-editor-left-panel-worksheet';
import { SqlEditorLeftPanelWorksheetsSkeleton } from './sql-editor-left-panel-worksheets-skeleton';

interface WorksheetsProps {
  worksheets: Worksheet[];
}

function Worksheets({ worksheets }: WorksheetsProps) {
  return worksheets.map((worksheet) => (
    <SqlEditorLeftPanelWorksheet key={worksheet.id} worksheet={worksheet} />
  ));
}

interface SqlEditorLeftPanelWorksheetsProps {
  worksheets: Worksheet[];
  isFetchingWorksheets: boolean;
}

export function SqlEditorLeftPanelWorksheets({
  worksheets,
  isFetchingWorksheets,
}: SqlEditorLeftPanelWorksheetsProps) {
  return (
    // TODO: Hardcode
    <ScrollArea className="h-[calc(100%-56px-2px)] py-2">
      <SidebarMenu className="flex w-full flex-col px-2">
        {isFetchingWorksheets ? (
          <SqlEditorLeftPanelWorksheetsSkeleton />
        ) : (
          <Worksheets worksheets={worksheets} />
        )}
      </SidebarMenu>
      <ScrollBar orientation="vertical" />
    </ScrollArea>
  );
}
