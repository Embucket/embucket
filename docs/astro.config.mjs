// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import tailwindcss from '@tailwindcss/vite';
import mermaid from 'astro-mermaid';
import starlightLinksValidator from 'starlight-links-validator';

// https://astro.build/config
export default defineConfig({
  integrations: [
    mermaid(), // Must come BEFORE starlight
    starlight({
      title: '',
      logo: {
        src: './src/assets/logo.svg',
      },
      favicon: '/favicon.ico',
      social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/embucket/embucket' }],
      sidebar: [
        {
          label: 'Getting Started',
          autogenerate: { directory: 'getting-started' },
        },
        {
          label: 'Deploy',
          autogenerate: { directory: 'deploy' },
        },
        {
          label: 'Connect',
          autogenerate: { directory: 'connect' },
        },
        {
          label: 'Tutorials',
          autogenerate: { directory: 'tutorials' },
        },
        {
          label: 'Reference',
          autogenerate: { directory: 'reference' },
        },
      ],
      customCss: ['./src/styles/global.css'],
      components: {
        ThemeSelect: './src/components/Empty.astro',
        ThemeProvider: './src/components/ForceDarkTheme.astro',
      },
      plugins: [
        starlightLinksValidator({
          errorOnLocalLinks: false,
        }),
      ],
    }),
  ],
  vite: {
    plugins: [tailwindcss()],
  },
  redirects: {
    '/': '/getting-started/quick-start/',
    '/essentials/quick-start/': '/getting-started/quick-start/',
    '/essentials/architecture/': '/reference/architecture/',
    '/essentials/configuration/': '/deploy/configuration/',
    '/essentials/runtime-modes/': '/deploy/aws-lambda/',
    '/essentials/snowflake/': '/reference/snowflake/',
    '/essentials/support-matrix/': '/deploy/aws-lambda/',
    '/guides/aws-lambda/': '/deploy/aws-lambda/',
    '/guides/dbt/': '/connect/dbt/',
    '/guides/snowflake-cli/': '/connect/snowflake-cli/',
    '/guides/snowplow/': '/tutorials/snowplow/',
    '/guides/troubleshooting/': '/reference/troubleshooting/',
    '/guides/s3-tables/': '/deploy/aws-lambda/',
    '/guides/self-hosted/': '/getting-started/quick-start/',
    '/guides/end-to-end-dbt/': '/connect/dbt/',
  },
});
