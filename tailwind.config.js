/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          0: "#1e1e2e",
          1: "#181825",
          2: "#11111b",
        },
        accent: {
          blue: "#89b4fa",
          green: "#a6e3a1",
          red: "#f38ba8",
          yellow: "#f9e2af",
          mauve: "#cba6f7",
          teal: "#94e2d5",
        },
        text: {
          primary: "#cdd6f4",
          secondary: "#a6adc8",
          muted: "#6c7086",
        },
      },
    },
  },
  plugins: [],
};
