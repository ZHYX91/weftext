import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({ variable: "--font-geist-sans", subsets: ["latin"] });
const geistMono = Geist_Mono({ variable: "--font-geist-mono", subsets: ["latin"] });

export const metadata: Metadata = {
  metadataBase: new URL("https://weftext-webui-prototype.zhengyx91.chatgpt.site"),
  title: "Weftext / 文缕 — 知识工作区原型",
  description: "面向本地与内网协作的原生 Rust 知识工作区。",
  openGraph: {
    title: "文缕 Weftext",
    description: "让知识有脉络，让协作有边界",
    type: "website",
    url: "https://weftext-webui-prototype.zhengyx91.chatgpt.site",
    images: [{ url: "/og.png", width: 1731, height: 907, alt: "文缕 Weftext — 让知识有脉络，让协作有边界" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "文缕 Weftext",
    description: "让知识有脉络，让协作有边界",
    images: ["/og.png"],
  },
  icons: { icon: "/app-icon.svg", shortcut: "/app-icon.svg", apple: "/app-icon.svg" },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="zh-CN"><body className={`${geistSans.variable} ${geistMono.variable}`}>{children}</body></html>;
}
