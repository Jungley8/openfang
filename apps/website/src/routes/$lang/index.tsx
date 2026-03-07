import { createFileRoute, Link } from '@tanstack/react-router';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { baseOptions } from '@/lib/layout.shared';
import {
  Shield,
  Cpu,
  Radio,
  Target,
  Bot,
  Zap,
} from 'lucide-react';

export const Route = createFileRoute('/$lang/')({
  component: Home,
});

const features = {
  cn: [
    {
      icon: Target,
      title: 'TELOS 目标系统',
      description: '基于 PAI 的个人目标引擎，让每个 Agent 理解你的使命、目标与挑战。',
    },
    {
      icon: Zap,
      title: 'Rust 构建，极致性能',
      description: '单一 ~32MB 二进制，<200ms 冷启动，40MB 空闲内存。14 个 Rust crate 协同工作。',
    },
    {
      icon: Shield,
      title: '16 层纵深安全',
      description: 'WASM 沙箱、Merkle 审计链、信息流追踪、Ed25519 签名、SSRF 防护、密钥零化。',
    },
    {
      icon: Radio,
      title: '40+ 渠道适配器',
      description: 'Telegram、Discord、Slack、微信（飞书）、Matrix、邮件等 40 个平台，开箱即用。',
    },
    {
      icon: Bot,
      title: '7 个自主 Hand',
      description: 'Researcher、Lead、Collector、Predictor、Twitter、Browser、Clip — 全天候为你工作。',
    },
    {
      icon: Cpu,
      title: '27 个 LLM 供应商',
      description: 'Anthropic、Gemini、OpenAI、DeepSeek、Groq、Ollama 等，智能路由与成本追踪。',
    },
  ],
  en: [
    {
      icon: Target,
      title: 'TELOS Goal System',
      description: 'PAI-powered personal goal engine. Every agent understands your mission, goals, and challenges.',
    },
    {
      icon: Zap,
      title: 'Built in Rust',
      description: 'Single ~32MB binary, <200ms cold start, 40MB idle memory. 14 Rust crates working together.',
    },
    {
      icon: Shield,
      title: '16-Layer Security',
      description: 'WASM sandbox, Merkle audit trail, taint tracking, Ed25519 signing, SSRF protection, secret zeroization.',
    },
    {
      icon: Radio,
      title: '40+ Channel Adapters',
      description: 'Telegram, Discord, Slack, Feishu/Lark, Matrix, Email and 34 more platforms, out of the box.',
    },
    {
      icon: Bot,
      title: '7 Autonomous Hands',
      description: 'Researcher, Lead, Collector, Predictor, Twitter, Browser, Clip — working for you 24/7.',
    },
    {
      icon: Cpu,
      title: '27 LLM Providers',
      description: 'Anthropic, Gemini, OpenAI, DeepSeek, Groq, Ollama and more. Intelligent routing with cost tracking.',
    },
  ],
};

const hero = {
  cn: {
    tagline: '有灵魂的 Agent OS',
    headline: '你的 Agent 知道该做什么\n但它知道为什么吗？',
    description:
      'Octarq 是一个自主 Agent 运行时，内置 TELOS 目标系统。每个 Agent 不只是执行任务 — 它理解你的使命，朝着你的目标推进。',
    cta: '快速开始',
    secondary: 'GitHub',
  },
  en: {
    tagline: 'The Agent OS with a Soul',
    headline: 'Your agents know what to do.\nBut do they know why?',
    description:
      'Octarq is an autonomous agent runtime powered by the TELOS goal system. Every agent doesn\'t just execute tasks — it understands your mission and moves your life forward.',
    cta: 'Quick Start',
    secondary: 'GitHub',
  },
};

function Home() {
  const { lang } = Route.useParams();
  const isEn = lang === 'en';
  const h = isEn ? hero.en : hero.cn;
  const f = isEn ? features.en : features.cn;

  return (
    <HomeLayout {...baseOptions(lang)}>
      {/* Hero */}
      <section className="flex flex-col items-center text-center px-4 pt-16 pb-12 md:pt-24 md:pb-16">
        <span className="text-sm font-medium text-fd-muted-foreground bg-fd-secondary px-3 py-1 rounded-full mb-6">
          {h.tagline}
        </span>
        <h1 className="text-4xl md:text-6xl font-bold tracking-tight whitespace-pre-line max-w-3xl">
          {h.headline}
        </h1>
        <p className="text-fd-muted-foreground text-lg md:text-xl max-w-2xl mt-6">
          {h.description}
        </p>
        <div className="flex gap-3 mt-8">
          <Link
            to="/$lang/docs/$"
            params={{ lang, _splat: 'getting-started' }}
            className="px-5 py-2.5 rounded-lg bg-fd-primary text-fd-primary-foreground font-medium text-sm"
          >
            {h.cta}
          </Link>
          <a
            href="https://github.com/Jungley8/Octarq"
            target="_blank"
            rel="noreferrer"
            className="px-5 py-2.5 rounded-lg border border-fd-border font-medium text-sm hover:bg-fd-secondary transition-colors"
          >
            {h.secondary}
          </a>
        </div>

        {/* Install command */}
        <div className="mt-8 bg-fd-secondary rounded-lg px-6 py-3 font-mono text-sm text-fd-muted-foreground">
          <span className="text-fd-foreground select-none">$ </span>
          curl -fsSL https://octarq.jungley.net/install.sh | sh
        </div>
      </section>

      {/* Features */}
      <section className="max-w-5xl mx-auto px-4 pb-16 md:pb-24">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {f.map((feature) => (
            <div
              key={feature.title}
              className="border border-fd-border rounded-xl p-6 hover:bg-fd-secondary/50 transition-colors"
            >
              <feature.icon className="size-8 text-fd-primary mb-4" />
              <h3 className="font-semibold text-lg mb-2">{feature.title}</h3>
              <p className="text-fd-muted-foreground text-sm leading-relaxed">
                {feature.description}
              </p>
            </div>
          ))}
        </div>
      </section>

      {/* Footer tagline */}
      <section className="text-center pb-16 text-fd-muted-foreground text-sm">
        Built with Rust. Secured with 16 layers.{' '}
        <span className="text-fd-foreground font-medium">
          {isEn ? 'Agents that actually work for you.' : '真正为你工作的 Agent。'}
        </span>
      </section>
    </HomeLayout>
  );
}
