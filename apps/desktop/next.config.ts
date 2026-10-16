import type { NextConfig } from "next";

const config: NextConfig = {
  output: "export",
  images: { unoptimized: true },
  transpilePackages: ["@boardmate/protocol", "@boardmate/sdk", "@boardmate/ui"],
  poweredByHeader: false,
};

export default config;
