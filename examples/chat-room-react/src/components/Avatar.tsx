export const Avatar = ({ emoji }: { emoji: string }) => {
  return <div className="avatar" data-emoji={emoji}>
    <div>{emoji}</div>
  </div>
}
