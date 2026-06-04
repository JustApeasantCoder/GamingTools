import type { ButtonHTMLAttributes, ComponentType } from 'react'
import { clsx } from 'clsx'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: ComponentType<{ size?: number }>
  variant?: 'default' | 'primary' | 'danger' | 'ghost'
}

export function Button({ children, icon: Icon, variant = 'default', className, ...props }: ButtonProps) {
  return (
    <button className={clsx('button', `button-${variant}`, className)} {...props}>
      {Icon ? <Icon size={17} /> : null}
      {children}
    </button>
  )
}
