import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { i18n } from '@/lib/i18n';

export const gitConfig = {
  user: 'Jungley8',
  repo: 'Octarq',
  branch: 'pro',
};

export function baseOptions(locale?: string): BaseLayoutProps {
  const isEn = locale === 'en';
  return {
    i18n,
    nav: {
      title: 'Octarq',
    },
    githubUrl: `https://github.com/${gitConfig.user}/${gitConfig.repo}`,
    links: [
      {
        text: isEn ? 'Documentation' : '文档',
        url: `/${locale ?? 'cn'}/docs`,
        active: 'nested-url',
      },
    ],
  };
}
