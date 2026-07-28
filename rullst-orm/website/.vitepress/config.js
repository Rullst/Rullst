import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "Rullst ORM",
  base: '/rullst-orm/',
  description: "A beautiful, type-safe, Active Record ORM for Rust.",
  themeConfig: {
    logo: '/vite.svg',
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Docs', link: '/docs/1-basics' },
      { text: 'Playground', link: '/playground' },
      { text: 'API Reference', link: 'https://docs.rs/rullst-orm' }
    ],
    sidebar: {
      '/docs/': [
        {
          text: 'Introduction',
          items: [
            { text: 'Basics & Query Builder', link: '/docs/1-basics' },
            { text: 'Relationships', link: '/docs/2-relationships' }
          ]
        },
        {
          text: 'Advanced',
          items: [
            { text: 'Advanced Features', link: '/docs/3-advanced-features' },
            { text: 'Migrations & Schema', link: '/docs/4-migrations-schema' },
            { text: 'Security & Testing', link: '/docs/5-security-and-testing' },
            { text: 'New Features (v6+)', link: '/docs/6-new-features' }
          ]
        }
      ]
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Rullst/rullst-orm' }
    ],
    footer: {
      message: 'Built with ❤️ for the Rust Community.',
      copyright: 'Copyright © 2026 Rullst ORM. Licensed under MIT.'
    },
    search: {
      provider: 'local'
    }
  },
  appearance: 'dark', // Force dark mode for that neon aesthetic
  ignoreDeadLinks: true
})
