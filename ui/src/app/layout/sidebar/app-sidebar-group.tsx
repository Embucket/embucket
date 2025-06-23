import type { LinkProps } from '@tanstack/react-router';
import { Link } from '@tanstack/react-router';
import type { LucideIcon } from 'lucide-react';

import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar';
import { cn } from '@/lib/utils';

interface SidebarNavOption {
  name: string;
  linkProps: LinkProps;
  Icon: LucideIcon;
  disabled?: boolean;
  onClick?: () => void;
  isActive?: boolean;
}

export function AppSidebarGroup({ items, open }: { items: SidebarNavOption[]; open: boolean }) {
  return (
    <SidebarGroup>
      <SidebarGroupContent className="px-0">
        <SidebarMenu>
          {items.map((item) => (
            <SidebarMenuItem key={item.name}>
              <Link
                onClick={(e) => {
                  if (item.onClick) {
                    e.preventDefault();
                    item.onClick();
                  }
                }}
                to={item.linkProps.to}
                params={item.linkProps.params}
                disabled={item.disabled}
                className="cursor-auto"
              >
                {({ isActive }) => (
                  <SidebarMenuButton
                    disabled={item.disabled}
                    className={cn(!isActive && 'hover:bg-hover!', 'text-nowrap')}
                    tooltip={{
                      children: item.name,
                      hidden: open,
                    }}
                    isActive={isActive || item.isActive}
                  >
                    <item.Icon />
                    {item.name}
                  </SidebarMenuButton>
                )}
              </Link>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarGroupContent>
    </SidebarGroup>
  );
}
