/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        tft: {
          gold: "#c8aa6e",
          dark: "#0a0a13",
          panel: "#1a1a2e",
          accent: "#0f3460",
          blue: "#16213e",
        },
      },
    },
  },
  plugins: [],
};
