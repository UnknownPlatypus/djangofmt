# Djangofmt Playground

A playground for djangofmt built using:

- [DaisyUI](https://daisyui.com/) & [TailwindCSS](https://tailwindcss.com/) for styling and light/dark themes
- [datastar](https://data-star.dev/) for frontend interactivity
- [Monaco Editor](https://github.com/microsoft/monaco-editor) for convenient code editor integration
- [Astro](https://astro.build/) to split HTML components

Why these technologies?

I really enjoy DaisyUI & datastar—they are awesome pieces of software.
They rely on the browser as much as possible and provide a declarative API, which is great for achieving a lot with little code.

The whole playground is ~700 LoC.

# Development

To run dev server:

```bash
just playground-dev
```

To build:

```bash
npm run build
```

To preview the build:

```bash
npm run preview
```
