import { VariantProps, tv } from "tailwind-variants";

const UserVariants = tv({
	slots: {
		base: "flex flex-col justify-center items-center cursor-pointer hover:text-primary-foreground rounded-lg",
		avatar: "size-8 rounded-lg",
	},
	variants: {
		size: {
			default: {
				base: "size-12",
				avatar: "size-8",
			},
			sm: {
				base: "size-10",
				avatar: "size-6",
			},
			lg: {
				base: "size-14",
				avatar: "size-10",
			},
			xl: {
				base: "size-16",
				avatar: "size-12",
			},
			"2xl": {
				base: "size-18",
				avatar: "size-14",
			},
			"3xl": {
				base: "size-20",
				avatar: "size-16",
			},
		},
	},
	defaultVariants: {
		size: "default",
	},
});

export interface UserProps
	extends React.HTMLAttributes<HTMLDivElement>,
		VariantProps<typeof UserVariants> {}

const User = ({ className, size, ...props }: UserProps) => {
	const { base, avatar } = UserVariants({ className, size });

	return (
		<div className={base()} {...props}>
			<div className={avatar()}>
				<img
					src="https://avatars.githubusercontent.com/u/19965768?v=4"
					alt=""
					className={avatar()}
				/>
			</div>
		</div>
	);
};
User.displayName = "User";

export { User, UserVariants };
