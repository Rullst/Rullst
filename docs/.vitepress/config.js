import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "Rullst Framework",
  description: "The Full-Stack Web Framework for Rust Language",
  appearance: 'dark', // force dark mode for that premium feel
  themeConfig: {
    logo: '/logo.png', // We will copy the Rullst.png here later
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/1-getting-started' },
      { text: 'Reference', link: '/reference/spec' },
      { text: 'Benchmarks', link: '/benches/' }
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'Getting Started', link: '/guide/1-getting-started' },
            { text: 'Philosophy', link: '/guide/philosophy' }
          ]
        },
        {
          text: 'Core Features',
          items: [
            { text: 'Rullst AI', link: '/guide/2-rullst-ai' },
            { text: 'Rullst Studio', link: '/guide/3-rullst-studio' },
            { text: 'Rullst Nexus', link: '/guide/4-rullst-nexus' },
            { text: 'Rullst Capital', link: '/guide/5-rullst-capital' }
          ]
        }
      ],
      '/reference/': [
        {
          text: 'Architecture',
          items: [
            { text: 'Framework Spec', link: '/reference/spec' },
            { text: 'Blueprints Roadmap', link: '/reference/blueprints_roadmap' }
          ]
        }
      ]
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Rullst/Rullst' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 Rullst Core Team'
    }
  }
})
