from setuptools import setup

setup(
    name="octarq",
    version="0.1.0",
    description="Official Python client for the Octarq Agent OS REST API",
    py_modules=["openfang_sdk", "openfang_client"],
    python_requires=">=3.8",
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
)
