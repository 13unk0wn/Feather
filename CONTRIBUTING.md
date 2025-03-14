# Contributing to FEATHER

Thank you for considering contributing to FEATHER! 🚀  
We welcome all contributions, whether it's fixing bugs, improving documentation, or adding new features.  

## 📌 Contribution Guidelines  

### ❗ Important Notice  
Feather v0.1 (`main` branch) **is only accepting bug fix pull requests**.  
- ✅ **Bug Fixes** → v0.1 (`main`)  
- ❌ **New Features, Enhancements, or Keybinding Changes** → v0.2 (`0.2` branch)  


### 1. Use the `dev` Branch for Pull Requests Made to the `main` Branch  
All contributions (except critical bug fixes for `main`) should be made to the `dev` branch.  
Before starting, make sure your local `dev` branch is up to date:  

### 2. You Can Directly Commit to the `v0.2` Branch for Additional Features  

```bash
git checkout dev
git pull origin dev
```

Always create a new feature branch for your changes:  

```bash
git checkout -b feature-branch
```

After making changes, commit and push:  

```bash
git add .
git commit -m "Describe your changes"
git push origin feature-branch
```

### 2. Submitting a Pull Request (PR)  
1. Go to the [GitHub repository](https://github.com/13unk0wn/Feather).  
2. Click **"New Pull Request"**.  
3. Ensure you are merging **your branch into the correct branch**:  
   - Bug fixes → `dev` (v0.1)  
   - New features, enhancements →  `0.2` (v0.2)  
4. Provide a clear PR description explaining the changes.  
5. Request a review from maintainers.  

Once approved, your PR will be merged into `dev`  and later into `main` for a stable release.  

### 3. Code Style & Best Practices  
- ✅ Follow the existing code style and formatting.  
- ✅ Write meaningful commit messages.  
- ✅ Keep PRs small and focused on a single feature/fix.  
- ✅ Test your code before submitting.  

### 4. Issues & Discussions  
- **Bug Reports:** If you find a bug, check if an issue already exists. Otherwise, create a new issue.  
- **Feature Requests:** If you have an idea, discuss it in an issue before implementing it.  

### 5. Need Help?  
If you're unsure about anything, feel free to open a discussion or ask in an issue. We're happy to help!  

🙌 **Happy Coding!** 🚀  
