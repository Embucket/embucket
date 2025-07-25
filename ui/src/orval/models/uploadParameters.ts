/**
 * Generated by orval v7.10.0 🍺
 * Do not edit manually.
 * UI Router API
 * Defines the specification for the UI Catalog API
 * OpenAPI spec version: 1.0.2
 */
import type { UploadParametersComment } from './uploadParametersComment';
import type { UploadParametersDelimiter } from './uploadParametersDelimiter';
import type { UploadParametersEscape } from './uploadParametersEscape';
import type { UploadParametersHeader } from './uploadParametersHeader';
import type { UploadParametersQuote } from './uploadParametersQuote';
import type { UploadParametersTerminator } from './uploadParametersTerminator';

export interface UploadParameters {
  /** @minimum 0 */
  comment?: UploadParametersComment;
  /** @minimum 0 */
  delimiter?: UploadParametersDelimiter;
  /** @minimum 0 */
  escape?: UploadParametersEscape;
  header?: UploadParametersHeader;
  /** @minimum 0 */
  quote?: UploadParametersQuote;
  /** @minimum 0 */
  terminator?: UploadParametersTerminator;
}
