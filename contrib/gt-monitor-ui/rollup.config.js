import resolve from "@rollup/plugin-node-resolve";
import terser from "@rollup/plugin-terser";

export default {
  input: "src/index.js",
  output: {
    file: "dist/gt-monitor-ui.js",
    format: "es",
    sourcemap: true,
  },
  plugins: [resolve(), terser()],
};
