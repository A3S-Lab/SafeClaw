import { useReactive } from "ahooks";
import { Check, Files } from "lucide-react";

const CopyButton = ({ content }: { content: string }) => {
    const state = useReactive({
        copied: false
    });
    return (
        <div
            className="flex items-center space-x-1 py-[4px] px-[8px] cursor-pointer group"
            onClick={() => {
                if (state.copied) return;
                navigator.clipboard.writeText(content);
                state.copied = true;
                setTimeout(() => {
                    state.copied = false;
                }, 2000);
            }}>
            {state.copied ? <>
                <Check className="group-hover:text-primary size-[14px] text-secondary-foreground/50" />
                <span className='group-hover:text-primary text-[12px] text-secondary-foreground/50'>完成</span>
            </> : <>
                <Files className="group-hover:text-primary size-[14px] text-secondary-foreground/50" />
                <span className='group-hover:text-primary text-[12px] text-secondary-foreground/50'>复制</span>
            </>}
        </div>
    )
}

export default CopyButton;