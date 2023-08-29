from typing import NewType, TypeVar

Message = TypeVar("Message")
CallbackName = NewType("CallbackName", str)
Command = NewType("Command", str)
