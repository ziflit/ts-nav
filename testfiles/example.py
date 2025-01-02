# This is a top level comment
def hello(arg1=None, arg2=None):
    print("hellooo")

def byee(arg1: str) -> None:
    print("goodbye")

def another():
    print("whatever")

class ATest:
    def amethod(self, arg1):
        return arg1
    def another_method(self):
        False
        return "irrelevant"

if __name__ == "__main__":
    hello()
    hello(None)
    hello(None, None)
    p = Prueba()
    p.amethod("hmmm")
