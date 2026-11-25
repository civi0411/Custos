import React, { useRef } from 'react';
import { CustomSelect, type CustomSelectOption } from './CustomSelect';

export function NativeSelect({ children, className, ...props }: React.SelectHTMLAttributes<HTMLSelectElement>) {
  const nativeRef = useRef<HTMLSelectElement>(null);
  const options: CustomSelectOption[] = [];
  const text = (node: React.ReactNode): string => React.Children.toArray(node).map(child =>
    React.isValidElement<{ children?: React.ReactNode }>(child) ? text(child.props.children) : String(child),
  ).join('');
  const collect = (nodes: React.ReactNode) => React.Children.forEach(nodes, node => {
    if (!React.isValidElement<React.OptionHTMLAttributes<HTMLOptionElement>>(node)) return;
    if (node.type === 'option') options.push({ value: String(node.props.value ?? text(node.props.children)), label: text(node.props.children), disabled: node.props.disabled });
    else collect(node.props.children);
  });
  collect(children);
  return <div className={`app-native-select min-w-0 ${className || ''}`}>
    <select {...props} ref={nativeRef} hidden tabIndex={-1} aria-hidden="true">{children}</select>
    <CustomSelect
      value={String(props.value ?? props.defaultValue ?? options[0]?.value ?? '')}
      options={options}
      disabled={props.disabled || options.every(option => option.disabled)}
      ariaLabel={props['aria-label'] || props.title}
      onChange={value => {
        const select = nativeRef.current;
        if (!select) return;
        select.value = String(value);
        // Keep the existing native change handlers and their event contract.
        select.dispatchEvent(new Event('change', { bubbles: true }));
      }}
    />
  </div>;
}
