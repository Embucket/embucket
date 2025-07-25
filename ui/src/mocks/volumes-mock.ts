import type { Volume } from '@/orval/models';

export const VOLUMES_MOCK: Volume[] = [
  {
    type: 'memory',
    name: 'myvolume',
    createdAt: '2021-01-01',
    updatedAt: '2021-01-01',
  },
  {
    type: 's3',
    name: 's3-volume',
    createdAt: '2021-01-01',
    updatedAt: '2021-01-01',
  },
];
