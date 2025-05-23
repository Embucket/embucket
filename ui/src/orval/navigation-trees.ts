/**
 * Generated by orval v7.9.0 🍺
 * Do not edit manually.
 * UI Router API
 * Defines the specification for the UI Catalog API
 * OpenAPI spec version: 1.0.2
 */
import { useInfiniteQuery, useQuery } from '@tanstack/react-query';
import type {
  DataTag,
  DefinedInitialDataOptions,
  DefinedUseInfiniteQueryResult,
  DefinedUseQueryResult,
  InfiniteData,
  QueryClient,
  QueryFunction,
  QueryKey,
  UndefinedInitialDataOptions,
  UseInfiniteQueryOptions,
  UseInfiniteQueryResult,
  UseQueryOptions,
  UseQueryResult,
} from '@tanstack/react-query';

import { useAxiosMutator } from '../lib/axiosMutator';
import type { ErrorType } from '../lib/axiosMutator';
import type { ErrorResponse, GetNavigationTreesParams, NavigationTreesResponse } from './models';

type SecondParameter<T extends (...args: never) => unknown> = Parameters<T>[1];

export const getNavigationTrees = (
  params?: GetNavigationTreesParams,
  options?: SecondParameter<typeof useAxiosMutator>,
  signal?: AbortSignal,
) => {
  return useAxiosMutator<NavigationTreesResponse>(
    { url: `/ui/navigation-trees`, method: 'GET', params, signal },
    options,
  );
};

export const getGetNavigationTreesQueryKey = (params?: GetNavigationTreesParams) => {
  return [`/ui/navigation-trees`, ...(params ? [params] : [])] as const;
};

export const getGetNavigationTreesInfiniteQueryOptions = <
  TData = InfiniteData<Awaited<ReturnType<typeof getNavigationTrees>>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<
      UseInfiniteQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
) => {
  const { query: queryOptions, request: requestOptions } = options ?? {};

  const queryKey = queryOptions?.queryKey ?? getGetNavigationTreesQueryKey(params);

  const queryFn: QueryFunction<Awaited<ReturnType<typeof getNavigationTrees>>> = ({ signal }) =>
    getNavigationTrees(params, requestOptions, signal);

  return { queryKey, queryFn, ...queryOptions } as UseInfiniteQueryOptions<
    Awaited<ReturnType<typeof getNavigationTrees>>,
    TError,
    TData
  > & { queryKey: DataTag<QueryKey, TData, TError> };
};

export type GetNavigationTreesInfiniteQueryResult = NonNullable<
  Awaited<ReturnType<typeof getNavigationTrees>>
>;
export type GetNavigationTreesInfiniteQueryError = ErrorType<ErrorResponse>;

export function useGetNavigationTreesInfinite<
  TData = InfiniteData<Awaited<ReturnType<typeof getNavigationTrees>>>,
  TError = ErrorType<ErrorResponse>,
>(
  params: undefined | GetNavigationTreesParams,
  options: {
    query: Partial<
      UseInfiniteQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    > &
      Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getNavigationTrees>>,
          TError,
          Awaited<ReturnType<typeof getNavigationTrees>>
        >,
        'initialData'
      >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): DefinedUseInfiniteQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };
export function useGetNavigationTreesInfinite<
  TData = InfiniteData<Awaited<ReturnType<typeof getNavigationTrees>>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<
      UseInfiniteQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    > &
      Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getNavigationTrees>>,
          TError,
          Awaited<ReturnType<typeof getNavigationTrees>>
        >,
        'initialData'
      >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseInfiniteQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };
export function useGetNavigationTreesInfinite<
  TData = InfiniteData<Awaited<ReturnType<typeof getNavigationTrees>>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<
      UseInfiniteQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseInfiniteQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

export function useGetNavigationTreesInfinite<
  TData = InfiniteData<Awaited<ReturnType<typeof getNavigationTrees>>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<
      UseInfiniteQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseInfiniteQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {
  const queryOptions = getGetNavigationTreesInfiniteQueryOptions(params, options);

  const query = useInfiniteQuery(queryOptions, queryClient) as UseInfiniteQueryResult<
    TData,
    TError
  > & { queryKey: DataTag<QueryKey, TData, TError> };

  query.queryKey = queryOptions.queryKey;

  return query;
}

export const getGetNavigationTreesQueryOptions = <
  TData = Awaited<ReturnType<typeof getNavigationTrees>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<UseQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>>;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
) => {
  const { query: queryOptions, request: requestOptions } = options ?? {};

  const queryKey = queryOptions?.queryKey ?? getGetNavigationTreesQueryKey(params);

  const queryFn: QueryFunction<Awaited<ReturnType<typeof getNavigationTrees>>> = ({ signal }) =>
    getNavigationTrees(params, requestOptions, signal);

  return { queryKey, queryFn, ...queryOptions } as UseQueryOptions<
    Awaited<ReturnType<typeof getNavigationTrees>>,
    TError,
    TData
  > & { queryKey: DataTag<QueryKey, TData, TError> };
};

export type GetNavigationTreesQueryResult = NonNullable<
  Awaited<ReturnType<typeof getNavigationTrees>>
>;
export type GetNavigationTreesQueryError = ErrorType<ErrorResponse>;

export function useGetNavigationTrees<
  TData = Awaited<ReturnType<typeof getNavigationTrees>>,
  TError = ErrorType<ErrorResponse>,
>(
  params: undefined | GetNavigationTreesParams,
  options: {
    query: Partial<UseQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>> &
      Pick<
        DefinedInitialDataOptions<
          Awaited<ReturnType<typeof getNavigationTrees>>,
          TError,
          Awaited<ReturnType<typeof getNavigationTrees>>
        >,
        'initialData'
      >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): DefinedUseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };
export function useGetNavigationTrees<
  TData = Awaited<ReturnType<typeof getNavigationTrees>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<
      UseQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>
    > &
      Pick<
        UndefinedInitialDataOptions<
          Awaited<ReturnType<typeof getNavigationTrees>>,
          TError,
          Awaited<ReturnType<typeof getNavigationTrees>>
        >,
        'initialData'
      >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };
export function useGetNavigationTrees<
  TData = Awaited<ReturnType<typeof getNavigationTrees>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<UseQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>>;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> };

export function useGetNavigationTrees<
  TData = Awaited<ReturnType<typeof getNavigationTrees>>,
  TError = ErrorType<ErrorResponse>,
>(
  params?: GetNavigationTreesParams,
  options?: {
    query?: Partial<UseQueryOptions<Awaited<ReturnType<typeof getNavigationTrees>>, TError, TData>>;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseQueryResult<TData, TError> & { queryKey: DataTag<QueryKey, TData, TError> } {
  const queryOptions = getGetNavigationTreesQueryOptions(params, options);

  const query = useQuery(queryOptions, queryClient) as UseQueryResult<TData, TError> & {
    queryKey: DataTag<QueryKey, TData, TError>;
  };

  query.queryKey = queryOptions.queryKey;

  return query;
}
