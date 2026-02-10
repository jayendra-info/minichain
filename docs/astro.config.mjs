// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import remarkMath from 'remark-math';
import rehypeKatex from 'rehype-katex';

// https://astro.build/config
export default defineConfig({
	site: 'https://jayendra-info.github.io',
	base: '/minichain',
	trailingSlash: 'ignore',
	markdown: {
		remarkPlugins: [remarkMath],
		rehypePlugins: [
			[
				rehypeKatex,
				{
					strict: false, // Handle Starlight edge cases
					trust: true,
					macros: {
						'\\Hash': '\\mathcal{H}',
						'\\Sig': '\\mathsf{Sig}',
						'\\Ver': '\\mathsf{Ver}',
					},
				},
			],
		],
	},
	integrations: [
		starlight({
			title: 'Building a Blockchain from Scratch',
			description: 'Learn how to build a minimal blockchain with Rust - from cryptographic primitives to a working CLI.',
			tableOfContents: {
				minHeadingLevel: 2,
				maxHeadingLevel: 3,
			},
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/jayendra-info/minichain' },
			],
			head: [
				{
					tag: 'script',
					attrs: {},
					content: `
						// Ensure all external links open in new tabs
						document.addEventListener('DOMContentLoaded', () => {
							const links = document.querySelectorAll('a[href^="http"], a[href^="https://"]');
							links.forEach(link => {
								// Skip if already has target attribute
								if (!link.getAttribute('target')) {
									link.setAttribute('target', '_blank');
									link.setAttribute('rel', 'noopener noreferrer');
								}
							});
						});
					`,
				},
			],
			sidebar: [
				{
					label: 'Introduction',
					items: [
						{ label: 'Welcome', slug: 'intro/welcome' },
						{ label: 'Project Overview', slug: 'intro/overview' },
					],
				},
				{
					label: 'Part 1: Foundation',
					items: [
						{ label: 'Chapter 1: Core Primitives', slug: 'part1/chapter1-core' },
					],
				},
				{
					label: 'Part 2: Storage',
					items: [
						{ label: 'Chapter 2: Persistent State', slug: 'part2/chapter2-storage' },
					],
				},
				{
					label: 'Part 3: Virtual Machine',
					items: [
						{ label: 'Chapter 3: Register-Based VM', slug: 'part3/chapter3-vm' },
					],
				},
				{
					label: 'Part 4: Assembler',
					items: [
						{ label: 'Chapter 4: Assembly to Bytecode', slug: 'part4/chapter4-assembler' },
					],
				},
				{
					label: 'Part 5: Blockchain',
					items: [
						{ label: 'Chapter 5: Consensus & Chain', slug: 'part5/chapter5-chain' },
					],
				},
				{
					label: 'Part 6: CLI',
					items: [
						{ label: 'Chapter 6: Command Line Interface', slug: 'part6/chapter6-cli' },
					],
				},
				{
					label: 'Appendices',
					items: [
						{ label: 'A: Elliptic Curve Taxonomy', slug: 'appendix/elliptic-curves' },
						{ label: 'B: Instruction Set Reference', slug: 'appendix/instruction-set' },
						{ label: 'C: Advanced Blockchain Concepts', slug: 'appendix/advanced-blockchain' },
					],
				},
			],
			customCss: ['./src/styles/custom.css'],
		}),
	],
});
