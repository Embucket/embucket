/* eslint-disable */

// @ts-nocheck

// noinspection JSUnusedGlobalSymbols

// This file was automatically generated by TanStack Router.
// You should NOT make any changes in this file as it will be overwritten.
// Additionally, you should also exclude this file from your linter and/or formatter to prevent it from being checked or modified.

import { createFileRoute } from '@tanstack/react-router';

import { Route as rootRouteImport } from './routes/__root';
import { Route as VolumesRouteImport } from './routes/volumes';
import { Route as HomeRouteImport } from './routes/home';
import { Route as IndexRouteImport } from './routes/index';
import { Route as QueriesIndexRouteImport } from './routes/queries/index';
import { Route as DatabasesDataPagesLayoutRouteImport } from './routes/databases/_dataPagesLayout';
import { Route as SqlEditorWorksheetIdIndexRouteImport } from './routes/sql-editor/$worksheetId.index';
import { Route as QueriesQueryIdIndexRouteImport } from './routes/queries/$queryId.index';
import { Route as DatabasesDataPagesLayoutIndexRouteImport } from './routes/databases/_dataPagesLayout.index';
import { Route as DatabasesDataPagesLayoutDatabaseNameSchemasIndexRouteImport } from './routes/databases/_dataPagesLayout.$databaseName.schemas.index';
import { Route as DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRouteImport } from './routes/databases/_dataPagesLayout.$databaseName.schemas.$schemaName.tables.index';
import { Route as DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRouteImport } from './routes/databases/_dataPagesLayout.$databaseName.schemas.$schemaName.tables.$tableName.data-preview.index';
import { Route as DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRouteImport } from './routes/databases/_dataPagesLayout.$databaseName.schemas.$schemaName.tables.$tableName.columns.index';

const DatabasesRouteImport = createFileRoute('/databases')();

const DatabasesRoute = DatabasesRouteImport.update({
  id: '/databases',
  path: '/databases',
  getParentRoute: () => rootRouteImport,
} as any);
const VolumesRoute = VolumesRouteImport.update({
  id: '/volumes',
  path: '/volumes',
  getParentRoute: () => rootRouteImport,
} as any);
const HomeRoute = HomeRouteImport.update({
  id: '/home',
  path: '/home',
  getParentRoute: () => rootRouteImport,
} as any);
const IndexRoute = IndexRouteImport.update({
  id: '/',
  path: '/',
  getParentRoute: () => rootRouteImport,
} as any);
const QueriesIndexRoute = QueriesIndexRouteImport.update({
  id: '/queries/',
  path: '/queries/',
  getParentRoute: () => rootRouteImport,
} as any);
const DatabasesDataPagesLayoutRoute =
  DatabasesDataPagesLayoutRouteImport.update({
    id: '/_dataPagesLayout',
    getParentRoute: () => DatabasesRoute,
  } as any);
const SqlEditorWorksheetIdIndexRoute =
  SqlEditorWorksheetIdIndexRouteImport.update({
    id: '/sql-editor/$worksheetId/',
    path: '/sql-editor/$worksheetId/',
    getParentRoute: () => rootRouteImport,
  } as any);
const QueriesQueryIdIndexRoute = QueriesQueryIdIndexRouteImport.update({
  id: '/queries/$queryId/',
  path: '/queries/$queryId/',
  getParentRoute: () => rootRouteImport,
} as any);
const DatabasesDataPagesLayoutIndexRoute =
  DatabasesDataPagesLayoutIndexRouteImport.update({
    id: '/',
    path: '/',
    getParentRoute: () => DatabasesDataPagesLayoutRoute,
  } as any);
const DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute =
  DatabasesDataPagesLayoutDatabaseNameSchemasIndexRouteImport.update({
    id: '/$databaseName/schemas/',
    path: '/$databaseName/schemas/',
    getParentRoute: () => DatabasesDataPagesLayoutRoute,
  } as any);
const DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute =
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRouteImport.update(
    {
      id: '/$databaseName/schemas/$schemaName/tables/',
      path: '/$databaseName/schemas/$schemaName/tables/',
      getParentRoute: () => DatabasesDataPagesLayoutRoute,
    } as any,
  );
const DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute =
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRouteImport.update(
    {
      id: '/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/',
      path: '/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/',
      getParentRoute: () => DatabasesDataPagesLayoutRoute,
    } as any,
  );
const DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute =
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRouteImport.update(
    {
      id: '/$databaseName/schemas/$schemaName/tables/$tableName/columns/',
      path: '/$databaseName/schemas/$schemaName/tables/$tableName/columns/',
      getParentRoute: () => DatabasesDataPagesLayoutRoute,
    } as any,
  );

export interface FileRoutesByFullPath {
  '/': typeof IndexRoute;
  '/home': typeof HomeRoute;
  '/volumes': typeof VolumesRoute;
  '/databases': typeof DatabasesDataPagesLayoutRouteWithChildren;
  '/queries': typeof QueriesIndexRoute;
  '/databases/': typeof DatabasesDataPagesLayoutIndexRoute;
  '/queries/$queryId': typeof QueriesQueryIdIndexRoute;
  '/sql-editor/$worksheetId': typeof SqlEditorWorksheetIdIndexRoute;
  '/databases/$databaseName/schemas': typeof DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables/$tableName/columns': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables/$tableName/data-preview': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute;
}
export interface FileRoutesByTo {
  '/': typeof IndexRoute;
  '/home': typeof HomeRoute;
  '/volumes': typeof VolumesRoute;
  '/databases': typeof DatabasesDataPagesLayoutIndexRoute;
  '/queries': typeof QueriesIndexRoute;
  '/queries/$queryId': typeof QueriesQueryIdIndexRoute;
  '/sql-editor/$worksheetId': typeof SqlEditorWorksheetIdIndexRoute;
  '/databases/$databaseName/schemas': typeof DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables/$tableName/columns': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute;
  '/databases/$databaseName/schemas/$schemaName/tables/$tableName/data-preview': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute;
}
export interface FileRoutesById {
  __root__: typeof rootRouteImport;
  '/': typeof IndexRoute;
  '/home': typeof HomeRoute;
  '/volumes': typeof VolumesRoute;
  '/databases': typeof DatabasesRouteWithChildren;
  '/databases/_dataPagesLayout': typeof DatabasesDataPagesLayoutRouteWithChildren;
  '/queries/': typeof QueriesIndexRoute;
  '/databases/_dataPagesLayout/': typeof DatabasesDataPagesLayoutIndexRoute;
  '/queries/$queryId/': typeof QueriesQueryIdIndexRoute;
  '/sql-editor/$worksheetId/': typeof SqlEditorWorksheetIdIndexRoute;
  '/databases/_dataPagesLayout/$databaseName/schemas/': typeof DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute;
  '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute;
  '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/columns/': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute;
  '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/': typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute;
}
export interface FileRouteTypes {
  fileRoutesByFullPath: FileRoutesByFullPath;
  fullPaths:
    | '/'
    | '/home'
    | '/volumes'
    | '/databases'
    | '/queries'
    | '/databases/'
    | '/queries/$queryId'
    | '/sql-editor/$worksheetId'
    | '/databases/$databaseName/schemas'
    | '/databases/$databaseName/schemas/$schemaName/tables'
    | '/databases/$databaseName/schemas/$schemaName/tables/$tableName/columns'
    | '/databases/$databaseName/schemas/$schemaName/tables/$tableName/data-preview';
  fileRoutesByTo: FileRoutesByTo;
  to:
    | '/'
    | '/home'
    | '/volumes'
    | '/databases'
    | '/queries'
    | '/queries/$queryId'
    | '/sql-editor/$worksheetId'
    | '/databases/$databaseName/schemas'
    | '/databases/$databaseName/schemas/$schemaName/tables'
    | '/databases/$databaseName/schemas/$schemaName/tables/$tableName/columns'
    | '/databases/$databaseName/schemas/$schemaName/tables/$tableName/data-preview';
  id:
    | '__root__'
    | '/'
    | '/home'
    | '/volumes'
    | '/databases'
    | '/databases/_dataPagesLayout'
    | '/queries/'
    | '/databases/_dataPagesLayout/'
    | '/queries/$queryId/'
    | '/sql-editor/$worksheetId/'
    | '/databases/_dataPagesLayout/$databaseName/schemas/'
    | '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/'
    | '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/columns/'
    | '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/';
  fileRoutesById: FileRoutesById;
}
export interface RootRouteChildren {
  IndexRoute: typeof IndexRoute;
  HomeRoute: typeof HomeRoute;
  VolumesRoute: typeof VolumesRoute;
  DatabasesRoute: typeof DatabasesRouteWithChildren;
  QueriesIndexRoute: typeof QueriesIndexRoute;
  QueriesQueryIdIndexRoute: typeof QueriesQueryIdIndexRoute;
  SqlEditorWorksheetIdIndexRoute: typeof SqlEditorWorksheetIdIndexRoute;
}

declare module '@tanstack/react-router' {
  interface FileRoutesByPath {
    '/databases': {
      id: '/databases';
      path: '/databases';
      fullPath: '/databases';
      preLoaderRoute: typeof DatabasesRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/volumes': {
      id: '/volumes';
      path: '/volumes';
      fullPath: '/volumes';
      preLoaderRoute: typeof VolumesRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/home': {
      id: '/home';
      path: '/home';
      fullPath: '/home';
      preLoaderRoute: typeof HomeRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/': {
      id: '/';
      path: '/';
      fullPath: '/';
      preLoaderRoute: typeof IndexRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/queries/': {
      id: '/queries/';
      path: '/queries';
      fullPath: '/queries';
      preLoaderRoute: typeof QueriesIndexRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/databases/_dataPagesLayout': {
      id: '/databases/_dataPagesLayout';
      path: '/databases';
      fullPath: '/databases';
      preLoaderRoute: typeof DatabasesDataPagesLayoutRouteImport;
      parentRoute: typeof DatabasesRoute;
    };
    '/sql-editor/$worksheetId/': {
      id: '/sql-editor/$worksheetId/';
      path: '/sql-editor/$worksheetId';
      fullPath: '/sql-editor/$worksheetId';
      preLoaderRoute: typeof SqlEditorWorksheetIdIndexRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/queries/$queryId/': {
      id: '/queries/$queryId/';
      path: '/queries/$queryId';
      fullPath: '/queries/$queryId';
      preLoaderRoute: typeof QueriesQueryIdIndexRouteImport;
      parentRoute: typeof rootRouteImport;
    };
    '/databases/_dataPagesLayout/': {
      id: '/databases/_dataPagesLayout/';
      path: '/';
      fullPath: '/databases/';
      preLoaderRoute: typeof DatabasesDataPagesLayoutIndexRouteImport;
      parentRoute: typeof DatabasesDataPagesLayoutRoute;
    };
    '/databases/_dataPagesLayout/$databaseName/schemas/': {
      id: '/databases/_dataPagesLayout/$databaseName/schemas/';
      path: '/$databaseName/schemas';
      fullPath: '/databases/$databaseName/schemas';
      preLoaderRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasIndexRouteImport;
      parentRoute: typeof DatabasesDataPagesLayoutRoute;
    };
    '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/': {
      id: '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/';
      path: '/$databaseName/schemas/$schemaName/tables';
      fullPath: '/databases/$databaseName/schemas/$schemaName/tables';
      preLoaderRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRouteImport;
      parentRoute: typeof DatabasesDataPagesLayoutRoute;
    };
    '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/': {
      id: '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/data-preview/';
      path: '/$databaseName/schemas/$schemaName/tables/$tableName/data-preview';
      fullPath: '/databases/$databaseName/schemas/$schemaName/tables/$tableName/data-preview';
      preLoaderRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRouteImport;
      parentRoute: typeof DatabasesDataPagesLayoutRoute;
    };
    '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/columns/': {
      id: '/databases/_dataPagesLayout/$databaseName/schemas/$schemaName/tables/$tableName/columns/';
      path: '/$databaseName/schemas/$schemaName/tables/$tableName/columns';
      fullPath: '/databases/$databaseName/schemas/$schemaName/tables/$tableName/columns';
      preLoaderRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRouteImport;
      parentRoute: typeof DatabasesDataPagesLayoutRoute;
    };
  }
}

interface DatabasesDataPagesLayoutRouteChildren {
  DatabasesDataPagesLayoutIndexRoute: typeof DatabasesDataPagesLayoutIndexRoute;
  DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute;
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute;
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute;
  DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute: typeof DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute;
}

const DatabasesDataPagesLayoutRouteChildren: DatabasesDataPagesLayoutRouteChildren =
  {
    DatabasesDataPagesLayoutIndexRoute: DatabasesDataPagesLayoutIndexRoute,
    DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute:
      DatabasesDataPagesLayoutDatabaseNameSchemasIndexRoute,
    DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute:
      DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesIndexRoute,
    DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute:
      DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameColumnsIndexRoute,
    DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute:
      DatabasesDataPagesLayoutDatabaseNameSchemasSchemaNameTablesTableNameDataPreviewIndexRoute,
  };

const DatabasesDataPagesLayoutRouteWithChildren =
  DatabasesDataPagesLayoutRoute._addFileChildren(
    DatabasesDataPagesLayoutRouteChildren,
  );

interface DatabasesRouteChildren {
  DatabasesDataPagesLayoutRoute: typeof DatabasesDataPagesLayoutRouteWithChildren;
}

const DatabasesRouteChildren: DatabasesRouteChildren = {
  DatabasesDataPagesLayoutRoute: DatabasesDataPagesLayoutRouteWithChildren,
};

const DatabasesRouteWithChildren = DatabasesRoute._addFileChildren(
  DatabasesRouteChildren,
);

const rootRouteChildren: RootRouteChildren = {
  IndexRoute: IndexRoute,
  HomeRoute: HomeRoute,
  VolumesRoute: VolumesRoute,
  DatabasesRoute: DatabasesRouteWithChildren,
  QueriesIndexRoute: QueriesIndexRoute,
  QueriesQueryIdIndexRoute: QueriesQueryIdIndexRoute,
  SqlEditorWorksheetIdIndexRoute: SqlEditorWorksheetIdIndexRoute,
};
export const routeTree = rootRouteImport
  ._addFileChildren(rootRouteChildren)
  ._addFileTypes<FileRouteTypes>();
