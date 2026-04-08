import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const docsRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(docsRoot, '..');

function read(relativePath, base = repoRoot) {
  return readFileSync(path.join(base, relativePath), 'utf8');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function assertIncludes(content, expected, label) {
  assert(content.includes(expected), `${label} is missing expected text: ${expected}`);
}

const quickStart = read('docs/src/content/docs/essentials/quick-start.mdx');
const snowflakeCli = read('docs/src/content/docs/guides/snowflake-cli.mdx');
const troubleshooting = read('docs/src/content/docs/guides/troubleshooting.mdx');
const runtimeModes = read('docs/src/content/docs/essentials/runtime-modes.mdx');
const configuration = read('docs/src/content/docs/essentials/configuration.mdx');
const supportMatrix = read('docs/src/content/docs/essentials/support-matrix.mdx');
const dbtGuide = read('docs/src/content/docs/guides/dbt.mdx');
const endToEndDbt = read('docs/src/content/docs/guides/end-to-end-dbt.mdx');
const docsMaintenance = read('docs/src/content/docs/development/docs-maintenance.mdx');
const awsLambda = read('docs/src/content/docs/guides/aws-lambda.mdx');
const s3Tables = read('docs/src/content/docs/guides/s3-tables.mdx');
const selfHosted = read('docs/src/content/docs/guides/self-hosted.mdx');
const snowplow = read('docs/src/content/docs/guides/snowplow.mdx');
const docsReadme = read('docs/README.md');
const packageJson = JSON.parse(read('docs/package.json'));

const highTrafficGuides = [
  'docs/src/content/docs/essentials/quick-start.mdx',
  'docs/src/content/docs/essentials/runtime-modes.mdx',
  'docs/src/content/docs/guides/aws-lambda.mdx',
  'docs/src/content/docs/guides/dbt.mdx',
  'docs/src/content/docs/guides/end-to-end-dbt.mdx',
  'docs/src/content/docs/guides/self-hosted.mdx',
  'docs/src/content/docs/guides/snowplow.mdx',
  'docs/src/content/docs/guides/troubleshooting.mdx',
];

for (const guidePath of highTrafficGuides) {
  const content = read(guidePath);
  assert(/> Owner:/m.test(content), `${guidePath} must include an Owner block near the top`);
  assert(
    /> Last reviewed:/m.test(content),
    `${guidePath} must include a Last reviewed block near the top`,
  );
}

for (const [content, label] of [
  [quickStart, 'Quick Start'],
  [snowflakeCli, 'Snowflake CLI guide'],
  [troubleshooting, 'Troubleshooting guide'],
]) {
  assertIncludes(content, 'embucket/embucket', label);
  assertIncludes(content, '3000', label);
  assertIncludes(content, 'embucket', label);
}

assertIncludes(quickStart, 'http://127.0.0.1:3000/', 'Quick Start');
assertIncludes(runtimeModes, 'private API Gateway example', 'Runtime modes');
assertIncludes(configuration, 'METASTORE_CONFIG=./metastore.yaml', 'Configuration guide');
assertIncludes(configuration, 'volumes: []', 'Configuration guide');
assertIncludes(snowflakeCli, 'protocol = "http"', 'Snowflake CLI guide');
assertIncludes(troubleshooting, 'protocol = "http"', 'Troubleshooting guide');
assertIncludes(selfHosted, 'cargo build', 'Self-hosted guide');
assertIncludes(selfHosted, 'target/debug/embucketd', 'Self-hosted guide');
assertIncludes(selfHosted, 'snow connection test', 'Self-hosted guide');
assertIncludes(selfHosted, 'evaluation and testing', 'Self-hosted guide');
assertIncludes(selfHosted, 'METASTORE_CONFIG=./metastore.yaml', 'Self-hosted guide');
assertIncludes(awsLambda, 'AWS::ApiGateway::RestApi', 'AWS Lambda guide');
assertIncludes(awsLambda, 'AWS::EC2::VPCEndpoint', 'AWS Lambda guide');
assertIncludes(snowplow, 'embucket-snowplow', 'Snowplow guide');
assertIncludes(snowplow, 'dbt run', 'Snowplow guide');
assertIncludes(snowplow, 'dbt show', 'Snowplow guide');
assertIncludes(snowplow, 'compatibility workaround', 'Snowplow guide');
assertIncludes(runtimeModes, '/guides/self-hosted/', 'Runtime modes');
assertIncludes(runtimeModes, '/guides/snowplow/', 'Runtime modes');
assertIncludes(supportMatrix, '/guides/self-hosted/', 'Support matrix');
assertIncludes(supportMatrix, '/guides/snowplow/', 'Support matrix');
assertIncludes(dbtGuide, '/guides/snowplow/', 'dbt guide');
assertIncludes(endToEndDbt, '/guides/snowplow/', 'End-to-end dbt guide');
assertIncludes(awsLambda, '/guides/snowplow/', 'AWS Lambda guide');
assertIncludes(s3Tables, '/guides/snowplow/', 'S3 Tables guide');
assertIncludes(docsMaintenance, 'Self-hosted local binary', 'Docs maintenance');
assertIncludes(docsMaintenance, 'Snowplow web analytics', 'Docs maintenance');

const requiredCommands = [
  'pnpm dev',
  'pnpm build',
  'pnpm preview',
  'pnpm astro',
  'pnpm format',
  'pnpm ncu',
];
for (const command of requiredCommands) {
  assertIncludes(docsReadme, command, 'docs/README.md');
}

for (const scriptName of ['astro', 'build', 'check', 'format', 'ncu', 'preview', 'smoke']) {
  assert(scriptName in packageJson.scripts, `docs/package.json is missing script: ${scriptName}`);
}

console.log('Docs smoke checks passed.');
