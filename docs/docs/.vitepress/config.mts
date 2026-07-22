import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "Rullst Framework",
  description: "The Ultimate Full-Stack Web Framework for Rust",
  themeConfig: {
    logo: 'https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png',
    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/getting-started' }
    ],
    sidebar: [
      {
        text: 'Introduction',
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'Core Concepts', link: '/core-concepts' }
        ]
      }
    ],
    socialLinks: [
      { icon: 'github', link: 'https://github.com/venelouis/Rullst' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026-present Rullst Core Team'
    }
  }
})
