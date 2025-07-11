/**
 * Generated by orval v7.10.0 🍺
 * Do not edit manually.
 * UI Router API
 * Defines the specification for the UI Catalog API
 * OpenAPI spec version: 1.0.2
 */
import type { TablePreviewDataParametersLimit } from './tablePreviewDataParametersLimit';
import type { TablePreviewDataParametersOffset } from './tablePreviewDataParametersOffset';

export interface TablePreviewDataParameters {
  /** @minimum 0 */
  limit?: TablePreviewDataParametersLimit;
  /** @minimum 0 */
  offset?: TablePreviewDataParametersOffset;
}
