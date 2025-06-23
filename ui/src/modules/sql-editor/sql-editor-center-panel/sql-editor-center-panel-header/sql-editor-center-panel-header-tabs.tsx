import { Link } from '@tanstack/react-router';
import { SquareTerminal } from 'lucide-react';

import { cn } from '@/lib/utils';

import { useSqlEditorSettingsStore } from '../../sql-editor-settings-store';
import { SqlEditorCenterPanelHeaderTabsCloseButton } from './sql-editor-center-panel-header-tabs-close-button';

export function SqlEditorCenterPanelHeaderTabs() {
  const tabs = useSqlEditorSettingsStore((state) => state.tabs);

  return (
    <div className="relative flex items-center gap-1">
      {tabs.map((tab) => (
        <Link
          key={tab.id}
          to="/sql-editor/$worksheetId"
          params={{ worksheetId: tab.id.toString() }}
        >
          {({ isActive }) => (
            <div
              className={cn(
                'bg-muted relative flex h-9 w-[180px] items-center self-end rounded-tl-md rounded-tr-md rounded-b-none border border-b-0 px-3 text-xs',
                'hover:bg-hover',
                isActive
                  ? 'text-primary-foreground bg-transparent hover:bg-transparent'
                  : 'border-none',
              )}
            >
              <SquareTerminal
                className={cn(
                  'text-muted-foreground mr-2 size-4 min-h-4 min-w-4 justify-start',
                  isActive && 'text-primary-foreground',
                )}
              />
              <span className="mr-2 max-w-28 truncate">{tab.name}</span>
              <SqlEditorCenterPanelHeaderTabsCloseButton tab={tab} />
            </div>
          )}
        </Link>
      ))}
    </div>
  );
}
