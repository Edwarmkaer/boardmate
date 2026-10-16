import type { Metadata } from "next";
import type { ReactNode } from "react";
import "@boardmate/ui/styles.css";
import "./panel.css";

export const metadata: Metadata = {
  title: "boardmate",
  description: "Un compañero desktop para tu hardware.",
  icons: { icon: "/icon.svg" },
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="es">
      <body>{children}</body>
    </html>
  );
}
