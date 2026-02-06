import { ReactNode } from 'react';
import CopyButton from '../copy-button';

interface CodeHighlightProps {
	className?: string;
	children?: ReactNode;
}

const CodeHighlight = ({ className, children, ...props }: CodeHighlightProps) => {
	const code = String(children).trim();
	const language = className?.match(/language-(\w+)/)?.[1];
	const isInline = !className;

	return !isInline ? (
		<div className="relative group">
			<pre className={className} {...props}>
				<code>{children}</code>
			</pre>
			<CopyButton text={code} />
		</div>
	) : (
		<code className={className} {...props}>
			{children}
		</code>
	);
};

export default CodeHighlight;
