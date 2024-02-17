export type Stream<T> = {
	subscribe: ({ on_data, on_end }: {
		on_data?: (data: T) => void,
		on_end?: () => void,
	}) => void
};
