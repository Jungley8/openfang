import {
  createRootRoute,
  HeadContent,
  Outlet,
  Scripts,
  useParams,
} from '@tanstack/react-router';
import * as React from 'react';
import appCss from '@/styles/app.css?url';
import { RootProvider } from 'fumadocs-ui/provider/base';
import { TanstackProvider } from 'fumadocs-core/framework/tanstack';
import { defineI18nUI } from 'fumadocs-ui/i18n';
import { i18n } from '@/lib/i18n';

const { provider } = defineI18nUI(i18n, {
  translations: {
    cn: {
      displayName: '中文',
      search: '搜索文档',
      searchNoResult: '没有找到结果',
      toc: '目录',
      lastUpdate: '最后更新',
      previousPage: '上一页',
      nextPage: '下一页',
      chooseLanguage: '选择语言',
    },
    en: {
      displayName: 'English',
    },
  },
});

export const Route = createRootRoute({
  head: () => ({
    meta: [
      {
        charSet: 'utf-8',
      },
      {
        name: 'viewport',
        content: 'width=device-width, initial-scale=1',
      },
      {
        title: 'Octarq — 有灵魂的 Agent OS',
      },
    ],
    links: [
      { rel: 'stylesheet', href: appCss },
    ],
  }),
  component: RootComponent,
});

function RootComponent() {
  return (
    <RootDocument>
      <Outlet />
    </RootDocument>
  );
}

function RootDocument({ children }: { children: React.ReactNode }) {
  const { lang } = useParams({ strict: false });

  return (
    <html lang={lang ?? 'cn'} suppressHydrationWarning>
      <head>
        <HeadContent />
      </head>
      <body className="flex flex-col min-h-screen">
        <TanstackProvider>
          <RootProvider i18n={provider(lang)}>
            {children}
          </RootProvider>
        </TanstackProvider>
        <Scripts />
      </body>
    </html>
  );
}
