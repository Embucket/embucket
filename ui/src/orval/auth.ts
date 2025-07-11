/**
 * Generated by orval v7.10.0 🍺
 * Do not edit manually.
 * UI Router API
 * Defines the specification for the UI Catalog API
 * OpenAPI spec version: 1.0.2
 */
import { useMutation } from '@tanstack/react-query';
import type {
  MutationFunction,
  QueryClient,
  UseMutationOptions,
  UseMutationResult,
} from '@tanstack/react-query';

import { useAxiosMutator } from '../lib/axiosMutator';
import type { ErrorType } from '../lib/axiosMutator';
import type { AuthErrorResponse, AuthResponse, LoginPayload } from './models';

type SecondParameter<T extends (...args: never) => unknown> = Parameters<T>[1];

export const login = (
  loginPayload: LoginPayload,
  options?: SecondParameter<typeof useAxiosMutator>,
  signal?: AbortSignal,
) => {
  return useAxiosMutator<AuthResponse>(
    {
      url: `/ui/auth/login`,
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      data: loginPayload,
      signal,
    },
    options,
  );
};

export const getLoginMutationOptions = <
  TError = ErrorType<AuthErrorResponse>,
  TContext = unknown,
>(options?: {
  mutation?: UseMutationOptions<
    Awaited<ReturnType<typeof login>>,
    TError,
    { data: LoginPayload },
    TContext
  >;
  request?: SecondParameter<typeof useAxiosMutator>;
}): UseMutationOptions<
  Awaited<ReturnType<typeof login>>,
  TError,
  { data: LoginPayload },
  TContext
> => {
  const mutationKey = ['login'];
  const { mutation: mutationOptions, request: requestOptions } = options
    ? options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey
      ? options
      : { ...options, mutation: { ...options.mutation, mutationKey } }
    : { mutation: { mutationKey }, request: undefined };

  const mutationFn: MutationFunction<Awaited<ReturnType<typeof login>>, { data: LoginPayload }> = (
    props,
  ) => {
    const { data } = props ?? {};

    return login(data, requestOptions);
  };

  return { mutationFn, ...mutationOptions };
};

export type LoginMutationResult = NonNullable<Awaited<ReturnType<typeof login>>>;
export type LoginMutationBody = LoginPayload;
export type LoginMutationError = ErrorType<AuthErrorResponse>;

export const useLogin = <TError = ErrorType<AuthErrorResponse>, TContext = unknown>(
  options?: {
    mutation?: UseMutationOptions<
      Awaited<ReturnType<typeof login>>,
      TError,
      { data: LoginPayload },
      TContext
    >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseMutationResult<
  Awaited<ReturnType<typeof login>>,
  TError,
  { data: LoginPayload },
  TContext
> => {
  const mutationOptions = getLoginMutationOptions(options);

  return useMutation(mutationOptions, queryClient);
};
export const logout = (options?: SecondParameter<typeof useAxiosMutator>, signal?: AbortSignal) => {
  return useAxiosMutator<void>({ url: `/ui/auth/logout`, method: 'POST', signal }, options);
};

export const getLogoutMutationOptions = <
  TError = ErrorType<AuthErrorResponse>,
  TContext = unknown,
>(options?: {
  mutation?: UseMutationOptions<Awaited<ReturnType<typeof logout>>, TError, void, TContext>;
  request?: SecondParameter<typeof useAxiosMutator>;
}): UseMutationOptions<Awaited<ReturnType<typeof logout>>, TError, void, TContext> => {
  const mutationKey = ['logout'];
  const { mutation: mutationOptions, request: requestOptions } = options
    ? options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey
      ? options
      : { ...options, mutation: { ...options.mutation, mutationKey } }
    : { mutation: { mutationKey }, request: undefined };

  const mutationFn: MutationFunction<Awaited<ReturnType<typeof logout>>, void> = () => {
    return logout(requestOptions);
  };

  return { mutationFn, ...mutationOptions };
};

export type LogoutMutationResult = NonNullable<Awaited<ReturnType<typeof logout>>>;

export type LogoutMutationError = ErrorType<AuthErrorResponse>;

export const useLogout = <TError = ErrorType<AuthErrorResponse>, TContext = unknown>(
  options?: {
    mutation?: UseMutationOptions<Awaited<ReturnType<typeof logout>>, TError, void, TContext>;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseMutationResult<Awaited<ReturnType<typeof logout>>, TError, void, TContext> => {
  const mutationOptions = getLogoutMutationOptions(options);

  return useMutation(mutationOptions, queryClient);
};
export const refreshAuthToken = (
  options?: SecondParameter<typeof useAxiosMutator>,
  signal?: AbortSignal,
) => {
  return useAxiosMutator<AuthResponse>(
    { url: `/ui/auth/refresh`, method: 'POST', signal },
    options,
  );
};

export const getRefreshAuthTokenMutationOptions = <
  TError = ErrorType<AuthErrorResponse>,
  TContext = unknown,
>(options?: {
  mutation?: UseMutationOptions<
    Awaited<ReturnType<typeof refreshAuthToken>>,
    TError,
    void,
    TContext
  >;
  request?: SecondParameter<typeof useAxiosMutator>;
}): UseMutationOptions<Awaited<ReturnType<typeof refreshAuthToken>>, TError, void, TContext> => {
  const mutationKey = ['refreshAuthToken'];
  const { mutation: mutationOptions, request: requestOptions } = options
    ? options.mutation && 'mutationKey' in options.mutation && options.mutation.mutationKey
      ? options
      : { ...options, mutation: { ...options.mutation, mutationKey } }
    : { mutation: { mutationKey }, request: undefined };

  const mutationFn: MutationFunction<Awaited<ReturnType<typeof refreshAuthToken>>, void> = () => {
    return refreshAuthToken(requestOptions);
  };

  return { mutationFn, ...mutationOptions };
};

export type RefreshAuthTokenMutationResult = NonNullable<
  Awaited<ReturnType<typeof refreshAuthToken>>
>;

export type RefreshAuthTokenMutationError = ErrorType<AuthErrorResponse>;

export const useRefreshAuthToken = <TError = ErrorType<AuthErrorResponse>, TContext = unknown>(
  options?: {
    mutation?: UseMutationOptions<
      Awaited<ReturnType<typeof refreshAuthToken>>,
      TError,
      void,
      TContext
    >;
    request?: SecondParameter<typeof useAxiosMutator>;
  },
  queryClient?: QueryClient,
): UseMutationResult<Awaited<ReturnType<typeof refreshAuthToken>>, TError, void, TContext> => {
  const mutationOptions = getRefreshAuthTokenMutationOptions(options);

  return useMutation(mutationOptions, queryClient);
};
